interface=`route | grep default | awk '{print $8}'`
sudo iptables -A FORWARD -m state --state RELATED,ESTABLISHED -j ACCEPT
sudo iptables -A FORWARD -i tun0 -o $interface -j ACCEPT
sudo iptables -t nat -A POSTROUTING -o $interface -j MASQUERADE
sudo sysctl net.ipv4.ip_forward=1