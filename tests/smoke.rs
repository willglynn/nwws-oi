use futures::StreamExt;
use nwws_oi::StreamEvent;
use std::time::Duration;

#[tokio::test]
async fn smoke_test() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .filter_module("nwws_oi", log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let (username, password) = match (
        std::env::var("NWWS_OI_USERNAME"),
        std::env::var("NWWS_OI_PASSWORD"),
    ) {
        (Ok(user), Ok(pass)) if !user.is_empty() && !pass.is_empty() => (user, pass),
        _ => {
            log::warn!("NWWS_OI_USERNAME and NWWS_OI_PASSWORD must be set");
            log::warn!("Skipping smoke test");
            return;
        }
    };

    let stream = nwws_oi::Stream::new((username, password));

    let received_test_message = stream.any(|event| {
        futures::future::ready(match event {
            StreamEvent::ConnectionState(_state) => false,
            StreamEvent::Error(error) => {
                log::error!("error: {:?}", error);
                false
            }
            StreamEvent::Message(message) => {
                log::info!("rx: {}.{}", message.ttaaii, message.cccc);
                if message.ttaaii == "WOUS99" && message.cccc == "KNCF" {
                    // THIS IS A COMMUNICATIONS TEST MESSAGE ORIGINATING FROM THE ANCF
                    true
                } else {
                    // ignore
                    false
                }
            }
        })
    });

    match tokio::time::timeout(Duration::from_secs(75), received_test_message).await {
        Ok(true) => log::info!("received ANCF test message from NWWS OI"),
        Ok(false) => unreachable!("stream ended"),
        Err(_) => panic!("timed out without receiving ANCF test message"),
    }
}
