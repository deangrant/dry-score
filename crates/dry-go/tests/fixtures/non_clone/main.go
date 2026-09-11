package fixtures

func computeSum(values []int) int {
	total := 0
	for _, value := range values {
		total += value
	}
	if total < 0 {
		return 0
	}
	return total
}

func computeProduct(values []int) int {
	total := 1
	for _, value := range values {
		if value == 0 {
			return 0
		}
		total *= value
	}
	return total
}
