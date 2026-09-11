package fixtures

func nearLeft(value int, weight int) int {
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

func nearRight(value int, weight int) int {
	acc := value
	if acc < 0 {
		acc = 0 - acc
	}
	product := acc * weight
	if product > 100 {
		return product - 10
	}
	bonus := 2
	return product + bonus
}
