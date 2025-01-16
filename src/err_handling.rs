use std::fmt::Debug;
use std::sync::Arc;

use futures::future::BoxFuture;
use teloxide::error_handlers::ErrorHandler;

pub struct MyErrorHandler {
    text: String,
}

impl MyErrorHandler {
    pub fn with_custom_text<T>(text: T) -> Arc<Self>
    where
        T: Into<String>,
    {
        Arc::new(Self { text: text.into() })
    }

    pub fn new() -> Arc<Self> {
        Self::with_custom_text("Error".to_owned())
    }
}

impl<E> ErrorHandler<E> for MyErrorHandler
where
    E: Debug,
{
    fn handle_error(self: Arc<Self>, error: E) -> BoxFuture<'static, ()> {
        let error_text = format!("{text}: {:?}", error, text = self.text);
        log::error!("{}", &error_text);

        Box::pin(async {})

        //log_error(error_text).boxed()
    }
}

// pub async fn log_error(text: String) {
//     let _ = BOT_CONFIG
//         .bot
//         .send_message(ChatId(BOT_CONFIG.log_group_id), text)
//         .await;
// }
