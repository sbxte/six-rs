while true; do
	date
	RUST_BACKTRACE=full ./target/release/sixclient
	sleep 300
done
