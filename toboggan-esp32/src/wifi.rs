use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{modem::Modem, peripheral},
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::info;

/// Initialize synchronous `WiFi` connection with provided credentials
///
/// # Errors
/// Returns error if `WiFi` initialization fails, SSID/password conversion fails,
/// scanning fails, or connection to the specified network fails
///
/// # Panics  
/// Panics if SSID or password cannot be converted to `WiFi` configuration format
#[allow(clippy::min_ident_chars)]
pub fn wifi_sync(
    ssid: &str,
    password: &str,
    modem: impl peripheral::Peripheral<P = Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>> {
    let mut auth_method = AuthMethod::WPA2Personal;
    if ssid.is_empty() {
        bail!("Missing WiFi name");
    }
    if password.is_empty() {
        auth_method = AuthMethod::None;
        info!("WiFi password is empty");
    }

    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    info!("Starting WiFi...");
    wifi.start()?;

    info!("Scanning...");
    let ap_infos = wifi.scan()?;
    let target_ap = ap_infos.into_iter().find(|ap| ap.ssid == ssid);
    let channel = if let Some(target_ap) = target_ap {
        info!(
            "Found configured access point {ssid} on channel {}",
            target_ap.channel
        );
        Some(target_ap.channel)
    } else {
        info!("Configured access point {ssid} not found during scanning, will go with unknown channel");
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .map_err(|()| anyhow::anyhow!("Failed to parse SSID into WiFi config"))?,
        password: password
            .try_into()
            .map_err(|()| anyhow::anyhow!("Failed to parse password into WiFi config"))?,
        channel,
        auth_method,
        ..Default::default()
    }))?;

    info!("Connecting WiFi...");
    wifi.connect()?;

    info!("Waiting for DHCP lease...");
    wifi.wait_netif_up()?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("WiFi DHCP info: {ip_info:?}");

    Ok(Box::new(esp_wifi))
}
