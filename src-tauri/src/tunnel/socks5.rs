use anyhow::{anyhow, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const VER: u8 = 0x05;
const CMD_CONNECT: u8 = 0x01;
const REP_SUCCESS: u8 = 0x00;
const REP_CONN_REFUSED: u8 = 0x05;
const REP_CMD_NOT_SUPPORTED: u8 = 0x07;
const REP_ATYP_NOT_SUPPORTED: u8 = 0x08;

/// Send a SOCKS5 reply (VER REP RSV ATYP=IPv4 0.0.0.0:0).
async fn send_reply<S: AsyncWrite + Unpin>(stream: &mut S, rep: u8) -> Result<()> {
    let buf = [VER, rep, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    stream.write_all(&buf).await?;
    stream.flush().await?;
    Ok(())
}

/// Tell the client the CONNECT succeeded.
pub async fn reply_success<S: AsyncWrite + Unpin>(stream: &mut S) -> Result<()> {
    send_reply(stream, REP_SUCCESS).await
}

/// Tell the client the CONNECT failed (host unreachable / refused).
pub async fn reply_failure<S: AsyncWrite + Unpin>(stream: &mut S) -> Result<()> {
    send_reply(stream, REP_CONN_REFUSED).await
}

/// Perform a no-auth SOCKS5 handshake and read the CONNECT request, returning
/// the requested target `(host, port)`. Does NOT send the final CONNECT reply —
/// the caller sends `reply_success`/`reply_failure` after attempting the tunnel.
pub async fn handshake<S>(stream: &mut S) -> Result<(String, u16)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // ── Greeting: VER NMETHODS METHODS ──
    let mut head = [0u8; 2];
    stream.read_exact(&mut head).await?;
    if head[0] != VER {
        return Err(anyhow!("unsupported SOCKS version: {}", head[0]));
    }
    let nmethods = head[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await?;
    // Always select "no authentication".
    stream.write_all(&[VER, 0x00]).await?;
    stream.flush().await?;

    // ── Request: VER CMD RSV ATYP DST.ADDR DST.PORT ──
    let mut req = [0u8; 4];
    stream.read_exact(&mut req).await?;
    if req[0] != VER {
        return Err(anyhow!("bad SOCKS request version: {}", req[0]));
    }
    if req[1] != CMD_CONNECT {
        let _ = send_reply(stream, REP_CMD_NOT_SUPPORTED).await;
        return Err(anyhow!("unsupported SOCKS command: {}", req[1]));
    }

    let host = match req[3] {
        0x01 => {
            let mut a = [0u8; 4];
            stream.read_exact(&mut a).await?;
            format!("{}.{}.{}.{}", a[0], a[1], a[2], a[3])
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut d = vec![0u8; len[0] as usize];
            stream.read_exact(&mut d).await?;
            String::from_utf8_lossy(&d).into_owned()
        }
        0x04 => {
            let mut a = [0u8; 16];
            stream.read_exact(&mut a).await?;
            a.chunks(2)
                .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
                .collect::<Vec<_>>()
                .join(":")
        }
        other => {
            let _ = send_reply(stream, REP_ATYP_NOT_SUPPORTED).await;
            return Err(anyhow!("unsupported SOCKS address type: {}", other));
        }
    };

    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await?;
    Ok((host, u16::from_be_bytes(port)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn ipv4_connect() {
        let (mut client, mut server) = tokio::io::duplex(64);
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap(); // greeting
        // CONNECT 1.2.3.4:80
        client
            .write_all(&[0x05, 0x01, 0x00, 0x01, 1, 2, 3, 4, 0x00, 0x50])
            .await
            .unwrap();

        let (host, port) = handshake(&mut server).await.unwrap();
        assert_eq!(host, "1.2.3.4");
        assert_eq!(port, 80);

        // Client should have received the no-auth method selection.
        let mut sel = [0u8; 2];
        client.read_exact(&mut sel).await.unwrap();
        assert_eq!(sel, [0x05, 0x00]);
    }

    #[tokio::test]
    async fn domain_connect() {
        let (mut client, mut server) = tokio::io::duplex(64);
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        let domain = b"example.com";
        let mut req = vec![0x05, 0x01, 0x00, 0x03, domain.len() as u8];
        req.extend_from_slice(domain);
        req.extend_from_slice(&443u16.to_be_bytes());
        client.write_all(&req).await.unwrap();

        let (host, port) = handshake(&mut server).await.unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
    }

    #[tokio::test]
    async fn rejects_non_connect() {
        let (mut client, mut server) = tokio::io::duplex(64);
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        // BIND command (0x02) — unsupported
        client
            .write_all(&[0x05, 0x02, 0x00, 0x01, 1, 2, 3, 4, 0x00, 0x50])
            .await
            .unwrap();

        assert!(handshake(&mut server).await.is_err());
    }
}
