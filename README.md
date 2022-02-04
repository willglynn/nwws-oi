# `nwws-oi`

A Rust client for the [NOAA Weather Wire Service](https://www.weather.gov/nwws/) Open Interface.

NWWS-OI is [one of several](https://www.weather.gov/nwws/dissemination) platforms through which the National Weather
Service distributes text products. The NWS operates two XMPP servers which push [text
products](https://forecast.weather.gov/product_types.php?site=NWS) in realtime to clients over the Internet as they are
published.

## Features

* `#![forbid(unsafe_code)]`
* Pure Rust
* Async (using [Tokio](https://tokio.rs))

## Example

```rust
let mut stream = nwws_oi::Stream::new((username, password));

while let Some(event) = stream.next().await {
    match event {
        StreamEvent::ConnectionState(state) => {}
        StreamEvent::Error(error) => {},
        StreamEvent::Message(message) => {},
    }
}
```

## Quickstart

1. [Sign up](https://www.weather.gov/nwws/nwws_oi_request)
2. Receive credentials from the NWS by email
3. 

```console
$ export NWWS_USERNAME=x    # <-- change
$ export NWWS_PASSWORD=y    # <-- change
$ cargo run --example print
    Finished dev [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/examples/print`
[2022-02-04T01:04:56Z INFO  nwws_oi] connecting to nwws-oi.weather.gov
[2022-02-04T01:04:59Z INFO  nwws_oi] connected to NWWS-OI x@nwws-oi.weather.gov/uuid/cda49512-b62a-46e9-87a4-1e1dd12b910b and joined MUC as NWWS@conference.nwws-oi.weather.gov/x/uuid/cda49512-b62a-46e9-87a4-1e1dd12b910b
[2022-02-04T01:05:00Z INFO  print] received Message {
        ttaaii: "SXUS74",
        cccc: "KOUN",
        awips_id: Some(
            "REROKC",
        ),
        issue: 2022-02-04T00:47:00+00:00,
        id: "14425.22326",
        delay_stamp: Some(
            2022-02-04T01:05:32.484+00:00,
        ),
        message: "\n109\nSXUS74 KOUN 040104\nREROKC\n\nRECORD EVENT REPORT\nNATIONAL WEATHER SERVICE NORMAN OK\n0647 PM CST THU FEB 03 2022\n\n...RECORD DAILY MAXIMUM SNOWFALL SET AT OKLAHOMA CITY...\n\nA RECORD SNOWFALL OF 3.3 INCHES HAS FALLEN SO FAR AT WILL ROGERS \nWORLD AIRPORT OKLAHOMA CITY.  THIS BREAKS THE OLD RECORD OF 1.0 SET \nON THIS DATE IN 1913. SNOW IS STILL FALLING IN OKLAHOMA CITY AT THIS \nTIME OF THIS ISSUANCE. ANOTHER REPORT WILL BE ISSUED WITH THE END OF \nTHE DAY SNOWFALL TOTALS.\n\n$$\n",
    }
[2022-02-04T01:05:02Z INFO  print] received Message {
        ttaaii: "WOUS99",
        cccc: "KNCF",
        awips_id: None,
        issue: 2022-02-04T01:05:00+00:00,
        id: "14425.22327",
        delay_stamp: None,
        message: "\n110\nWOUS99 KNCF 040105\n@pil\nTHIS IS A COMMUNICATIONS TEST MESSAGE ORIGINATING FROM THE ANCF\nSYSTEM IN SILVER SPRING.  IT IS SET TO TRANSMIT EVERY MINUTE\n24 HOURS PER DAY, SEVEN DAYS PER WEEK, VIA A CRON JOB ON THE\nCS1F-ANCF.ER SERVER.  IT IS SENT TO NCF WHERE IT IS ROUTED\nEITHER VIA SBN OR WAN TO ALL AWIPS SITES. THIS PRODUCT IS USED\nFOR PERFORMANCE MEASUREMENTS AND WILL BE RECEIVED\nOVER THE WAN (NCFHPTNCF) OR SBN (NCFWTSNCF and NCFTSTNCF) BY ALL SITES.\n",
    }
```
