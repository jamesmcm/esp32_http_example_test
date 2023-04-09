#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
                      // use esp_idf_svc::wifi;
use embedded_hal_async::delay::DelayUs;
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::{EspNetif, NetifConfiguration},
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
};

use embassy_time::{Duration, Timer};
use embedded_svc::{
    http::{client::asynch::Client as HttpClient, Method, Status},
    io::asynch::Write,
    utils::io::asynch,
};
use esp_idf_svc::http::client_async::{Configuration as HttpConfiguration, EspHttpConnection};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    println!("start");
    esp_idf_sys::link_patches();
    println!("start1");
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("start2");
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    println!("start3");
    // let config = Configuration::Client(ClientConfiguration {
    //     ssid: "".into(),
    //     bssid: None,
    //     auth_method: embedded_svc::wifi::AuthMethod::WPA2Personal,
    //     password: "".into(),
    //     channel: None,
    // });

    let config = Configuration::Client(ClientConfiguration {
        ssid: "Wokwi-GUEST".into(),
        bssid: None,
        auth_method: embedded_svc::wifi::AuthMethod::None,
        password: "".into(),
        channel: None,
    });
    println!("start4");
    let mut wifi = EspWifi::new(peripherals.modem, sys_loop, Some(nvs)).unwrap();
    println!("start5");
    wifi.set_configuration(&config).unwrap();

    println!("start6");
    wifi.start().unwrap();
    println!("start7");
    wifi.connect().unwrap();
    while !wifi.is_connected().unwrap() {
        let config = wifi.get_configuration().unwrap();
        println!("Waiting for station {:?}", config);
    }
    println!("Should be connected now");

    println!("IP info: {:?}", wifi.sta_netif().get_ip_info().unwrap());

    // TODO: Async await for wifi to be ready

    Timer::after(Duration::from_secs(2)).await;

    println!("IP info: {:?}", wifi.sta_netif().get_ip_info().unwrap());

    spawner.spawn(task()).unwrap();
    println!("Spawned task");
    // Delay {}.delay_ms(7000).await.unwrap();
    spawner.spawn(run()).unwrap();
    println!("Delay finished");
}

#[embassy_executor::task]
async fn run() {
    loop {
        println!("tick");
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn task() {
    // Create HTTP(S) client
    println!("Inside task");
    let mut client = HttpClient::wrap(
        EspHttpConnection::new(&HttpConfiguration {
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach), // Needed for HTTPS support
            ..Default::default()
        })
        .unwrap(),
    );

    // GET
    get_request(&mut client).await.unwrap();

    println!("Hello, world!");
}

async fn get_request(client: &mut HttpClient<EspHttpConnection>) -> anyhow::Result<()> {
    // Prepare headers and URL
    let headers = [("accept", "text/plain"), ("connection", "close")];
    let url = "https://www.bbc.com/news";

    // Send request
    //
    // Note: If you don't want to pass in any headers, you can also use `client.get(url, headers)`.
    let request = client.request(Method::Get, &url, &headers).await?;
    println!("-> GET {}", url);
    let mut response = request.submit().await?;

    // Process response
    let status = response.status();
    println!("<- {}", status);
    println!();
    let (_headers, mut body) = response.split();
    let mut buf = [0u8; 1024];
    let bytes_read = asynch::try_read_full(&mut body, &mut buf)
        .await
        .map_err(|e| e.0)?;
    println!("Read {} bytes", bytes_read);
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => println!(
            "Response body (truncated to {} bytes): {:?}",
            buf.len(),
            body_string
        ),
        Err(e) => eprintln!("Error decoding response body: {}", e),
    };

    // Drain the remaining response bytes
    while body.read(&mut buf).await? > 0 {}

    Ok(())
}
