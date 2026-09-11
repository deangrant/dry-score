package fixtures

func alphaPath(value int, weight int) int {
	acc := value
	if acc < 0 {
		acc = 0 - acc
	}
	product := acc * weight
	if product > 100 {
		return product - 10
	}
	return product + 1
}

func betaPath(amount int, multiplier int) int {
	running := amount
	if running < 0 {
		running = 0 - running
	}
	product := running * multiplier
	if product > 100 {
		return product - 10
	}
	return product + 1
}
