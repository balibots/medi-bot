use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use teloxide::types::InlineKeyboardButton;

use crate::{frequency::Frequency, patient::Patient};
use redis::{Commands, Connection, RedisError};
use redis_macros::{FromRedisValue, ToRedisArgs};

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Medication {
    id: String,
    pub patient_id: String,
    pub medicine: String,
    pub dosage: String,
    frequency: Frequency,
    user_id: String,
    pub last_taken: Option<i64>,
    pub patient_name: Option<String>,
}

impl Medication {
    pub fn new(
        patient_id: String,
        medicine: String,
        dosage: String,
        frequency: Frequency,
        user_id: String,
    ) -> Medication {
        Medication {
            id: uuid::Uuid::new_v4().to_string().replace("-", ""),
            patient_id,
            medicine,
            dosage,
            frequency,
            user_id,
            last_taken: None,
            patient_name: None,
        }
    }

    pub fn save(&mut self, connection: Arc<Mutex<Connection>>) -> Result<(), RedisError> {
        println!("saving {:?}", self);

        let patient = Patient::get_by_id(&self.patient_id, connection.clone());

        if let Ok(p) = patient {
            self.patient_name = Some(p.name)
        }

        let mut con = connection.lock().unwrap();

        match con.set::<String, &Medication, ()>(format!("medi:{}", self.id), self) {
            Ok(result) => {
                println!("saved {:?}", result);
            }
            Err(error) => {
                println!("error {:?}", error);
            }
        };

        con.sadd::<String, String, ()>(
            format!("medi:patient_meds:{}", self.patient_id.to_string()),
            self.id.to_string(),
        )
        .expect("Error adding medication to patient set array");

        Ok(())
    }

    pub fn set_taken_now(&mut self, connection: Arc<Mutex<Connection>>) -> Result<(), RedisError> {
        self.last_taken = Some(Utc::now().timestamp());

        connection
            .clone()
            .lock()
            .unwrap()
            .lpush::<String, i64, ()>(
                format!("medi:{}:taken", self.id),
                self.last_taken.unwrap(),
            )?;

        self.save(connection)
    }

    pub fn get_medication_log(
        &self,
        connection: Arc<Mutex<Connection>>,
    ) -> Result<Vec<i64>, RedisError> {
        connection
            .clone()
            .lock()
            .unwrap()
            .lrange::<String, Vec<i64>>(format!("medi:{}:taken", self.id), 0, 10)
    }

    pub fn can_take(&self) -> bool {
        if self.last_taken.is_none() {
            true
        } else {
            let lt = DateTime::from_timestamp(self.last_taken.unwrap(), 0).unwrap();
            lt + TimeDelta::hours(self.frequency.get_hours().into()) < Utc::now()
        }
    }

    pub fn can_take_emoji(&self) -> String {
        if self.can_take() {
            "✅".to_string()
        } else {
            "🙅".to_string()
        }
    }

    fn print_can_take_next(&self) -> String {
        if self.can_take() {
            "Right now".to_string()
        } else {
            let lt = DateTime::from_timestamp(self.last_taken.unwrap(), 0).unwrap()
                + TimeDelta::hours(self.frequency.get_hours().into());
            let now = Utc::now();
            let dif = lt - now;
            if dif.num_hours() > 0 {
                format!(
                    "in {} hours and {} minutes",
                    dif.num_hours().to_string(),
                    (dif.checked_sub(&TimeDelta::hours(dif.num_hours())))
                        .unwrap()
                        .num_minutes()
                )
            } else {
                format!("in {} minutes", dif.num_minutes().to_string())
            }
        }
    }

    pub fn print_in_list(&self) -> String {
        let can_take = if self.can_take() { "✅" } else { "🙅" };

        format!(
            "{} ({}) - {}. Last taken: {}. Can take next: {} {}.",
            self.medicine,
            self.dosage,
            self.frequency, // TODO implement display
            self.print_last_taken(),
            self.print_can_take_next(),
            can_take
        )
    }

    pub fn print_last_taken(&self) -> String {
        match self.last_taken {
            None => "Not yet".to_string(),
            Some(ts) => {
                let date = DateTime::from_timestamp(ts, 0).unwrap();
                let now = Utc::now();
                let dif = now - date;
                match dif.num_hours() {
                    h if h > 0 && h < 24 => format!("{} hours ago", dif.num_hours()),
                    h if h == 0 && dif.num_minutes() > 0 => {
                        format!("{} minutes ago", dif.num_minutes())
                    }
                    _ if dif.num_minutes() == 0 => format!("Just now"),
                    _ => date.to_string(),
                }
            }
        }
    }

    pub fn get_by_id(id: &str, con: Arc<Mutex<Connection>>) -> Result<Self, RedisError> {
        con.lock()
            .unwrap()
            .get::<String, Medication>(format!("medi:{}", id))
    }

    pub fn get_all_by_patient_id(patient_id: &str, con: Arc<Mutex<Connection>>) -> Vec<Medication> {
        let ids = con
            .lock()
            .unwrap()
            .smembers::<String, Vec<String>>(format!("medi:patient_meds:{}", patient_id))
            .unwrap();

        ids.into_iter()
            .map(|id| Medication::get_by_id(&id, con.clone()))
            .filter_map(|m| if m.is_ok() { Some(m.unwrap()) } else { None })
            .collect::<Vec<Medication>>()
    }

    pub fn generate_medication_keyboard(
        patient_id: &str,
        con: Arc<Mutex<Connection>>,
    ) -> Vec<Vec<InlineKeyboardButton>> {
        let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

        let medications = Medication::get_all_by_patient_id(patient_id, con);

        for medication_chunk in medications.chunks(2) {
            let row = medication_chunk
                .iter()
                .map(|med| {
                    InlineKeyboardButton::callback(
                        format!("{} {} ({})", med.can_take_emoji(), med.medicine, med.dosage),
                        med.id.clone(),
                    )
                })
                .collect();

            keyboard.push(row);
        }

        keyboard.push(vec![InlineKeyboardButton::callback(
            "Cancel".to_string(),
            "cancel".to_string(),
        )]);

        keyboard
    }
}

/*
  PLAN:
    SET: medi:{id} { id, name, medicine, dosage, frequencyH, last_taken, user_id}
    SADD: user:user_id  [medi:{id}, ...]

    SADD: medi:{id}:taken [timestamps ... ]

    ZSET: triggers: [ medi:id, 460000 (ts); ...]

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
            .arg(1) // selecting db 1 for tests to preserve data on the other one (default, 0)
            .exec(&mut redis_connection)
            .unwrap();
        redis::cmd("FLUSHDB").exec(&mut redis_connection).unwrap();

        redis_connection
    }

    #[test]
    fn test_create_save_medication() {
        let user_id = uuid::Uuid::new_v4().to_string();
        let patient = Patient::new("xavi".to_string(), user_id.clone());

        let mut medication = Medication::new(
            patient.id.clone(),
            "nurofen".to_string(),
            "5ml".to_string(),
            Frequency::new(3),
            user_id.clone(),
        );

        let redis_con = Arc::new(Mutex::new(create_redis_connection()));
        let res = medication.save(redis_con.clone());

        assert!(res.is_ok());

        let saved_medication = Medication::get_by_id(&medication.id, redis_con.clone()).unwrap();

        assert_eq!(saved_medication, medication);

        assert_eq!(
            redis_con
                .lock()
                .unwrap()
                .scard::<String, i32>(format!("medi:patient_meds:{}", patient.id))
                .unwrap(),
            1
        );
    }
}
