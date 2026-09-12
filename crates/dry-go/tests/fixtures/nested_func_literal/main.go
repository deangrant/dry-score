package fixtures

func withClosures() int {
	left := func(n int) int {
		acc := n
		if acc < 0 {
			acc = 0 - acc
		}
		return acc + 1
	}
	right := func(n int) int {
		acc := n
		if acc < 0 {
			acc = 0 - acc
		}
		return acc + 1
	}
	return left(3) + right(4)
}
