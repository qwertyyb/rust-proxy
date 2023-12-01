#!/bin/bash

sendUdp() {
  times=$1
  while (( $times > 0 ))
  do
    brook testsocks5 --socks5 127.0.0.1:7878 --dns "8.8.8.8:53"
    echo $times
    let "times--"
  done
}

sendHttp() {
  times=$1
  while (( $times > 0 ))
  do
    curl https://www.baidu.com -x socks5://127.0.0.1:7878
    echo $times
    let "times--"
  done
}

sendHttpWithHttpProxy() {
  times=$1
  while (( $times > 0 ))
  do
    curl https://www.baidu.com -x http://127.0.0.1:7878
    echo $times
    let "times--"
  done
}

# sendUdp 300

# sendHttp 300

sendHttpWithHttpProxy 300
