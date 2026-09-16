while true; do
	date
	RUST_BACKTRACE=full ../../target/release/example_tandai_hadir
	sleep 300
done
