# Minimal OpenBlink program.
#
# Hardware APIs (LED, GPIO, matrix, ...) are board-specific; replace the body
# below with the calls exposed by your board. This sample only prints to the
# device console so it builds and runs anywhere.
count = 0
loop do
  count += 1
  puts "blink #{count}"
  sleep 1
end
