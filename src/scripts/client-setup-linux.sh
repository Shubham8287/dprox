ip netns add dproxy
sudo ip link set tun0 netns dproxy
sudo ip netns exec dproxy bash
sudo ip address add 10.0.0.228/24 dev tun0
sudo ip link tun0 up
ip route add default via 100.0.0.228 dev tun0
adduser shu
xhost +SI:localuser:$USER

su shu