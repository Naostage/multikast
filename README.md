# Usage

To listen for data:
```
multikast -i <ipv4 or interface number> -a <multicast address> -p <udp port> -m listen
```

To talk:
```
multikast -i <ipv4 or interface number> -a <multicast address> -p <udp port> -m talk
```
it will send `stdin` lines to the multicast address.
