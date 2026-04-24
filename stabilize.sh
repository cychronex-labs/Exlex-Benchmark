echo "performance" | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
sync; echo 3 | sudo tee /proc/sys/vm/drop_caches