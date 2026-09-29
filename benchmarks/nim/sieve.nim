proc sieve(limit: int): int =
  var flags = newSeq[uint8](limit + 1)
  for i in 0..limit: flags[i] = 1
  flags[0] = 0
  flags[1] = 0
  var p = 2
  while p * p <= limit:
    if flags[p] != 0:
      var i = p * p
      while i <= limit:
        flags[i] = 0
        i += p
    p += 1
  var count = 0
  for i in 2..limit:
    if flags[i] != 0: count += 1
  return count

var total = 0
for iter in 0..<5:
  total = sieve(10_000_000)
echo total
