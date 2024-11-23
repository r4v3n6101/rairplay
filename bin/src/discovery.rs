use mdns_sd::{ServiceDaemon, ServiceInfo};

pub fn mdns_broadcast() {
    // TODO : remove hardcode
    let mdns = ServiceDaemon::new().expect("Could not create service daemon");
    let service_type = "_airplay._tcp.local.";
    let instance_name = "whateva";

    let my_addrs = "";
    let service_hostname = format!("{}{}", instance_name, &service_type);
    let port = 5200;

    let properties = [
        ("model", "rairplay"),
        ("protovers", "1.1"),
        ("srcvers", "366.0"),
        ("features", "0x405C4393,0x300"),
        ("deviceid", "9F:D7:AF:1F:D3:CD"),
    ];

    let service_info = ServiceInfo::new(
        service_type,
        instance_name,
        &service_hostname,
        my_addrs,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    mdns.register(service_info)
        .expect("Failed to register mDNS service");
}
