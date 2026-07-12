import socket
s = socket.socket()
s.bind(("127.0.0.1", 8080))
s.listen(1)
conn, addr = s.accept()
data = conn.recv(1024)
print(repr(data))
conn.send(b"HTTP/1.1 200 OK\r\n\r\nHello!")
conn.close()
