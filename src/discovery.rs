use airplay::config::Config;
use mdns_sd::{ServiceDaemon, ServiceInfo};

const SERVICE_TYPE: &str = "_airplay._tcp.local.";
const PROTOCOL_VERSION: &str = "1.1";

pub fn mdns_broadcast<ADev, VDev, KC>(config: &Config<ADev, VDev, KC>) {
    let mdns = ServiceDaemon::new().expect("Could not create service daemon");

    let instance_name = config.name.as_str();
    let service_hostname = format!("{}.local.", instance_name.replace(' ', "-"));
    let port = 5200;

    let feature_txt = format_feature_bits(config.features.bits());
    let device_id = config.mac_addr.to_string().to_uppercase();

    let properties = [
        ("model", "AppleTV3,2"),
        ("protovers", PROTOCOL_VERSION),
        ("srcvers", "366.0"),
        ("features", feature_txt.as_str()),
        ("deviceid", device_id.as_str()),
    ];

    let service_info = ServiceInfo::new(
        SERVICE_TYPE,
        instance_name,
        &service_hostname,
        "",
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    mdns.register(service_info)
        .expect("Failed to register mDNS service");
}

fn format_feature_bits(bits: u64) -> String {
    let lower = bits as u32;
    let upper = (bits >> 32) as u32;
    format!("0x{lower:08X},0x{upper:X}")
}
