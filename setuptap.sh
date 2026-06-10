#!/usr/bin/env bash

sudo ip link del tap0 2>/dev/null

sudo ip tuntap add dev tap0 mode tap

sudo ip addr add 192.168.10.1/24 dev tap0

# sudo ip link set tap0 master br0

sudo ip link set tap0 up


echo 1 | sudo tee /proc/sys/net/ipv4/ip_forward > /dev/null

sudo iptables -t nat -A POSTROUTING -s 192.168.10.0/24 -o br0 -j MASQUERADE

# nmcli con show --active
# sudo nmcli con add type bridge con-name br0 ifname br0
# sudo nmcli con add type bridge-slave con-name br0-slave ifname enp6s0 master br0
# sudo nmcli con down "enp6s0"
# sudo nmcli con up br0
