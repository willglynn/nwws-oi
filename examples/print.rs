use futures::StreamExt;
use nwws_oi::StreamEvent;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .filter_module("nwws_oi", log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let username = std::env::var("NWWS_USERNAME").expect("NWWS_USERNAME must be set");
    let password = std::env::var("NWWS_PASSWORD").expect("NWWS_PASSWORD must be set");

    let mut stream = nwws_oi::Stream::new((username, password));
    while let Some(event) = stream.next().await {
        match event {
            StreamEvent::ConnectionState(_state) => {}
            StreamEvent::Error(error) => log::error!("error: {}", format!("{:?}", error)),
            StreamEvent::Message(message) => {
                log::info!("{}", format!("{:#?}", message));
            }
        }
    }
}
