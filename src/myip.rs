use std::net::IpAddr;

pub trait IpAddressDiscovery {
    fn discover_ip_address(&self) -> Result<IpAddr, ()>;
}

pub struct AmazonCheckIP;

impl IpAddressDiscovery for AmazonCheckIP {
    fn discover_ip_address(&self) -> Result<IpAddr, ()> {
        use reqwest;

        let mut resp = reqwest::get("http://checkip.amazonaws.com").map_err(|_| ())?;

        if resp.status().is_success() {
            let body = resp.text().map_err(|_| ())?;
            body.trim().parse().map_err(|_| ())
        } else {
            Err(())
        }
    }
}

pub struct StaticIP(String);

impl StaticIP {
    pub fn new(ip: String) -> Self {
        StaticIP(ip)
    }
}

impl IpAddressDiscovery for StaticIP {
    fn discover_ip_address(&self) -> Result<IpAddr, ()> {
        self.0.parse().map_err(|_| ())
    }
}