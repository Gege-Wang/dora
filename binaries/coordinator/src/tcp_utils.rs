use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub async fn tcp_send(connection: &mut TcpStream, message: &[u8]) -> std::io::Result<()> {
    println!("**[coordinator]ready tcp_send: message: {:?}", message);
    let len_raw = (message.len() as u64).to_le_bytes();
    connection.write_all(&len_raw).await?;
    connection.write_all(message).await?;
    println!("**[coordinator]have sent tcp_send: message.len() as u64: {:?}, message is {:?}", len_raw, message);
    connection.flush().await?;
    Ok(())
}

pub async fn tcp_receive(connection: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let reply_len = {
        let mut raw = [0; 8];
        connection.read_exact(&mut raw).await?;
        u64::from_le_bytes(raw) as usize
    };
    let mut reply = vec![0; reply_len];
    connection.read_exact(&mut reply).await?;
    Ok(reply)
}
