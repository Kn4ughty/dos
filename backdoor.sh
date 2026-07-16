#!/bin/sh 

sudo nping --icmp 192.168.10.2 -c 1 --data $(nasm f.asm && xxd -p f | tr a-z A-z | sed 's/^/F00F/')
