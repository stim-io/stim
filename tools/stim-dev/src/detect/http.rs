use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

struct HttpEndpoint {
    host: String,
    port: u16,
    host_header: String,
}

pub(super) fn http_get_status(
    base_url: &str,
    path: &str,
    timeout: Duration,
) -> Result<u16, String> {
    let endpoint = parse_http_base_url(base_url)?;
    let address = format!("{}:{}", endpoint.host, endpoint.port);
    let socket_address = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}: no socket addresses"))?;
    let mut stream = TcpStream::connect_timeout(&socket_address, timeout)
        .map_err(|error| format!("failed to connect to {address}: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("failed to set read timeout for {address}: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("failed to set write timeout for {address}: {error}"))?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: stim-dev-detect\r\nConnection: close\r\n\r\n",
        endpoint.host_header
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write health request to {address}: {error}"))?;

    let mut buffer = [0; 512];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("failed to read health response from {address}: {error}"))?;
    if read == 0 {
        return Err(format!("health response from {address} was empty"));
    }

    let response = String::from_utf8_lossy(&buffer[..read]);
    let status_line = response
        .lines()
        .next()
        .ok_or_else(|| format!("health response from {address} had no status line"))?;
    status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("health response from {address} had malformed status line"))?
        .parse::<u16>()
        .map_err(|error| format!("health response from {address} had invalid status code: {error}"))
}

fn parse_http_base_url(base_url: &str) -> Result<HttpEndpoint, String> {
    let authority_and_path = base_url.strip_prefix("http://").ok_or_else(|| {
        format!("unsupported health URL scheme for {base_url}; only http:// is supported")
    })?;
    let authority = authority_and_path
        .split('/')
        .next()
        .filter(|authority| !authority.is_empty())
        .ok_or_else(|| format!("missing host in {base_url}"))?;

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port = port
                .parse::<u16>()
                .map_err(|error| format!("invalid port in {base_url}: {error}"))?;
            (host.to_string(), port)
        }
        None => (authority.to_string(), 80),
        Some(_) => return Err(format!("missing host in {base_url}")),
    };

    let host_header = if port == 80 {
        host.clone()
    } else {
        format!("{host}:{port}")
    };

    Ok(HttpEndpoint {
        host,
        port,
        host_header,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_http_base_url;

    #[test]
    fn parses_plain_http_base_urls_for_local_health_checks() {
        let endpoint = parse_http_base_url("http://127.0.0.1:18081").unwrap();

        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 18081);
        assert_eq!(endpoint.host_header, "127.0.0.1:18081");

        let endpoint = parse_http_base_url("http://localhost").unwrap();
        assert_eq!(endpoint.port, 80);
    }
}
