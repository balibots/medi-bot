use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::frequency::Frequency;
use redis::{Commands, Connection, RedisError};
use redis_macros::{FromRedisValue, ToRedisArgs};

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Medication {
    id: String,
    pub name: String,
    medicine: String,
    dosage: String,
    frequency: Frequency,
    user_id: String,
}

impl Medication {
    pub fn new(
        name: String,
        medicine: String,
        dosage: String,
        frequency: Frequency,
        user_id: String,
    ) -> Medication {
        Medication {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            medicine,
            dosage,
            frequency,
            user_id,
        }
    }
    pub fn save(&self, connection: Arc<Mutex<Connection>>) -> Result<(), RedisError> {
        println!("saving {:?}", self);

        let mut con = connection.lock().unwrap();

        match con.set::<String, &Medication, ()>(format!("medi:{}:{}", self.name, self.id), self) {
            Ok(result) => {
                println!("saved {:?}", result);
            }
            Err(error) => {
                println!("error {:?}", error);
            }
        };

        /* TODO: when should we do this ??? do we have a command for the first dose?
        con.zadd(
            "medi:trigger".to_string(), self.id, next_timestamp)
            */

        con.sadd::<String, String, ()>(
            format!("medi:user:{}", self.user_id.to_string()),
            self.id.to_string(),
        )
        .expect("Error adding medication to user set array");

        Ok(())
    }

    fn get_by_id_and_name(
        id: &str,
        name: &str,
        con: Arc<Mutex<Connection>>,
    ) -> Result<Self, RedisError> {
        con.lock()
            .unwrap()
            .get::<String, Medication>(format!("medi:{}:{}", name, id))
    }
}

/*
  PLAN:
    SET: medi:name:id { id, name, medicine, dosage, frequencyH, last_taken}
    ZSET: triggers: [ medi:id, 460000 (ts); ...]
    SET: user:user_id  [medi:id, ...]

    /next_dosages: map user_id set and return last_taken+freq? How to deal if it's distant past?
    /can_take <name>: map user_id, filter by name, return last_taken+freq in the past
    <trigger>: zrangerevby scores on triggers and notify
    /taken_id: update map, update trigger
*/

#[cfg(test)]

mod tests {
    use super::*;

    fn create_redis_connection() -> redis::Connection {
        // creating a real connection actually
        let client = redis::Client::open("redis://127.0.0.1/").expect("Could not connect to Redis");
        let mut redis_connection = client
            .get_connection()
            .expect("Could not get a Redis connection");

        redis::cmd("SELECT")
            .arg(1) // selecting db 1 for tests to preserve data on the other one (0)
            .exec(&mut redis_connection)
            .unwrap();
        redis::cmd("FLUSHDB").exec(&mut redis_connection).unwrap();

        redis_connection
    }

    #[test]
    fn test_create_save_medication() {
        let medication = Medication::new(
            "xavi".to_string(),
            "nurofen".to_string(),
            "5ml".to_string(),
            Frequency::new(3),
            "fake-id".to_string(),
        );

        let redis_con = Arc::new(Mutex::new(create_redis_connection()));
        let res = medication.save(redis_con.clone());

        assert!(res.is_ok());

        let saved_medication =
            Medication::get_by_id_and_name(&medication.id, &medication.name, redis_con.clone())
                .unwrap();

        assert_eq!(saved_medication, medication);

        assert_eq!(
            redis_con
                .lock()
                .unwrap()
                .scard::<String, i32>(format!("medi:user:{}", medication.user_id))
                .unwrap(),
            1
        );
    }
}
