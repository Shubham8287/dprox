# DPROX
Dprox is a distributed proxy, which is in development with intent to let clients easily create and connect to Virtual Private Networks over public internet.

## Arcitecture
Each private network is consist of one central server and any number of peers which once connected to central server, can each to any other client.

## Usage

Example command to start a server on local port 8080, Server should have public ip and should not behind the NAT  -

> cargo run server -p 8080

To connect a peer node to server
> cargo run client -s <server-ip> -p <server-port>

To get a info about network on a server
> cargo run info -s <server-ip> -p <server-port>
