proc mandelbrot(): int64 =
  let width = 600
  let height = 600
  let max_iter = 200
  var total: int64 = 0
  for py in 0..<height:
    let y0 = (py.float64 / height.float64) * 2.0 - 1.0
    for px in 0..<width:
      let x0 = (px.float64 / width.float64) * 3.0 - 2.0
      var x = 0.0
      var y = 0.0
      var iter = 0
      while (x * x + y * y <= 4.0) and (iter < max_iter):
        let xtemp = x * x - y * y + x0
        y = 2.0 * x * y + y0
        x = xtemp
        iter += 1
      total += iter
  return total

echo mandelbrot()
