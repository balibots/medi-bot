use std::sync::{Arc, Mutex};

use redis::{Commands, Connection, RedisError};
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use teloxide::types::InlineKeyboardButton;

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Patient {
    pub id: String,
    pub name: String,
    creator_user_id: String,
    shared_with: Vec<String>,
}

impl Patient {
    pub fn new(name: String, user_id: String) -> Patient {
        Patient {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            creator_user_id: user_id,
            shared_with: vec![],
        }
    }

    pub fn save(&self, connection: Arc<Mutex<Connection>>) -> Result<(), RedisError> {
        log::info!("saving patient {:?}", self);

        let mut con = connection.lock().unwrap();

        match con.set::<String, &Patient, ()>(format!("medi:patient:{}", self.id), self) {
            Ok(result) => {
                println!("saved {:?}", result);
            }
            Err(error) => {
                println!("error {:?}", error);
            }
        };

        con.sadd::<String, String, ()>(
            format!("medi:user_patient:{}", self.creator_user_id.to_string()),
            self.id.to_string(),
        )
        .expect("Error adding new patient to user set array");

        Ok(())
    }

    pub fn delete(&self, connection: Arc<Mutex<Connection>>) -> Result<(), RedisError> {
        log::info!("deleting patient {:?}", self);
        let mut con = connection.lock().unwrap();

        con.del::<String, ()>(format!("medi:patient:{}", self.id))
            .expect("Error deleting patient on del");

        for user_id in self.shared_with.iter() {
            con.srem::<String, String, ()>(
                format!("medi:user_patient:{}", user_id.to_string()),
                self.id.to_string(),
            )
            .expect("Error removing patient from user set array");
        }

        Ok(())
    }

    pub fn get_by_id(patient_id: &str, con: Arc<Mutex<Connection>>) -> Result<Self, RedisError> {
        log::info!("{}", patient_id);
        con.lock()
            .unwrap()
            .get::<String, Patient>(format!("medi:patient:{}", patient_id))
    }

    pub fn get_my_patients(
        user_id: &str,
        con: Arc<Mutex<Connection>>,
    ) -> Result<Vec<Self>, RedisError> {
        let ids = con
            .lock()
            .unwrap()
            .smembers::<String, Vec<String>>(format!("medi:user_patient:{}", user_id))
            .unwrap();

        Ok(ids
            .into_iter()
            .map(|id| Patient::get_by_id(&id, con.clone()))
            .filter_map(|m| if m.is_ok() { Some(m.unwrap()) } else { None })
            .collect::<Vec<Patient>>())
    }

    pub fn share(
        &mut self,
        telegram_user_id: &str,
        con: Arc<Mutex<Connection>>,
    ) -> Result<(), RedisError> {
        con.lock()
            .unwrap()
            .sadd::<String, String, ()>(
                format!("medi:user_patient:{}", telegram_user_id.to_string()),
                self.id.to_string(),
            )
            .expect("Error adding new patient to user set array");

        self.shared_with.push(telegram_user_id.to_string());

        Ok(())
    }

    pub fn generate_patient_keyboard(
        con: Arc<Mutex<Connection>>,
        user_id: String,
        show_add: bool,
    ) -> Vec<Vec<InlineKeyboardButton>> {
        let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

        let patients = Patient::get_my_patients(&user_id, con).unwrap();

        log::info!("{:?}", patients);

        for versions in patients.chunks(3) {
            let row = versions
                .iter()
                .map(|patient| {
                    InlineKeyboardButton::callback(patient.name.clone(), patient.id.clone())
                })
                .collect();

            keyboard.push(row);
        }

        if show_add {
            keyboard.push(vec![InlineKeyboardButton::callback(
                "Add new patient...".to_string(),
                "add_new".to_string(),
            )]);
        }

        keyboard.push(vec![InlineKeyboardButton::callback(
            "Cancel".to_string(),
            "cancel".to_string(),
        )]);

        keyboard
    }
}

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
            .arg(1) // selecting db 1 for tests to preserve data on the other one (default, 0)
            .exec(&mut redis_connection)
            .unwrap();
        redis::cmd("FLUSHDB").exec(&mut redis_connection).unwrap();

        redis_connection
    }

    #[test]
    fn test_create_patient() {
        let user_id = uuid::Uuid::new_v4().to_string();
        let patient = Patient::new("xavi".to_string(), user_id.clone());

        let redis_con = Arc::new(Mutex::new(create_redis_connection()));
        let res = patient.save(redis_con.clone());

        assert!(res.is_ok());

        let saved_patient = Patient::get_by_id(&patient.id, redis_con.clone()).unwrap();

        assert_eq!(patient, saved_patient);

        assert_eq!(
            redis_con
                .lock()
                .unwrap()
                .scard::<String, i32>(format!("medi:user_patient:{}", patient.creator_user_id))
                .unwrap(),
            1
        );
    }
}
