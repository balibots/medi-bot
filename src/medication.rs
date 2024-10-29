use std::{process::id, sync::{Arc, Mutex}};

use crate::{frequency::Frequency, MediBotPersistance};
use redis::{Commands, Connection, RedisError};

#[derive(Debug, PartialEq)]
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

        con.hset_multiple::<String, String, String, ()>(
            self.name.to_string(),
            &[("medicine".to_string(), self.medicine.to_string())],
        );

        con.zadd(id, member, score);

        con.sadd()



        Ok(())
    }
}

/*
    MAP: medi:id { id, name, medicine, dosage, frequencyH, last_taken}
    ZSET: triggers: [ medi:id, 460000 (ts); ...]
    SET: user:user_id  [medi:id, ...]

    /next_dosages: map user_id set and return last_taken+freq? How to deal if it's distant past?
    /can_take <name>: map user_id, filter by name, return last_taken+freq in the past
    <trigger>: zrangerevby scores on triggers and notify
    /taken_id: update map, update trigger
*/

#[cfg(test)]

mod tests {
    use std::collections::HashMap;

    use super::*;

    // struct MockPersistance {
    //     data: HashMap<String, String>,
    // }

    // impl MediBotPersistance for MockPersistance {
    //     fn new_medication(&mut self, m: &Medication) -> Result<(), ()> {
    //         self.data.insert(String::from(&m.name), "sim".to_string());
    //         Ok(())
    //     }
    //     fn get_patient(&mut self, name: &str) -> Result<String, ()> {
    //         if let Some(name) = self.data.get(name) {
    //             Ok(name.to_string())
    //         } else {
    //             Err(())
    //         }
    //     }
    // }

    fn create_mock_connection() -> Connection {
        // creating a real connection actually
        let client = redis::Client::open("redis://127.0.0.1/").expect("Could not connect to Redis");
        let mut redis_connection = client
            .get_connection()
            .expect("Could not get a Redis connection");

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

        let mock_connection = Arc::new(Mutex::new(create_mock_connection()));
        let res = medication.save(mock_connection.clone());

        assert!(res.is_ok());

        let m: Result<String, RedisError> = mock_connection
            .lock()
            .unwrap()
            .get::<&String, String>(&medication.name);

        assert_eq!(m.unwrap(), "asdasd");
    }
}
