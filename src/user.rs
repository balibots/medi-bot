use std::sync::{Arc, Mutex};

use redis::{Commands as _, Connection};

pub fn get_user_timezone(con: Arc<Mutex<Connection>>, user_id: &str) -> String {
    con.lock()
        .unwrap()
        .get::<String, String>(format!("medi:{}:timezone", user_id))
        .unwrap_or("UTC".to_string())
}
