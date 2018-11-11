#!/bin/sh

#set -x

cargo build --release

nc_hash() {
    local a=${1}
    local b=${2}
    ${a} -l -N 9999 | sha256sum > ${_hash_nc} &
    sleep 1
    cat ${temp_file} | ${b} -N localhost 9999
    wait

    hash_good=`cat ${_hash_good}`
    hash_nc=`cat ${_hash_nc}`
    if [ "${hash_good}" = "${hash_nc}" ]; then
        echo "# ${a} -> ${b} PASS"
    else
        echo "# ${a} -> ${b} FAIL"
    fi
}

integrity_check() {
    echo "### Integrity check"
    temp_file=$(mktemp)
    _hash_good=$(mktemp)
    _hash_nc=$(mktemp)

    dd if=/dev/urandom of=${temp_file} bs=1M count=16  > /dev/null 2>&1
    cat ${temp_file} | sha256sum > ${_hash_good}

    nc_hash nc nc
    nc_hash ./target/release/netcat nc
    nc_hash nc ./target/release/netcat
    nc_hash ./target/release/netcat ./target/release/netcat

    rm ${temp_file} ${_hash_good} ${_hash_nc}
}

COUNT=2000
BS=1M

nc_speed() {
    local a=${1}
    local b=${2}

    temp_file=$(mktemp)

    ${a} -l -N 9999 > /dev/null &
    sleep 1
    dd if=/dev/zero count=${COUNT} bs=${BS} 2> ${temp_file} | ${b} -N localhost 9999
    wait

    speed=`cat ${temp_file} 2>&1 | sed -n 3p | cut -d',' -f4`

    echo "# ${a} -> ${b}${speed}"
    #rm ${temp_file}
}

speed_test() {
    echo "### Speed test"

    nc_speed nc nc
    nc_speed ./target/release/netcat nc
    nc_speed nc ./target/release/netcat
    nc_speed ./target/release/netcat ./target/release/netcat
}

# nc -> nc
# rust -> nc
# nc -> rust

#integrity_check
speed_test
