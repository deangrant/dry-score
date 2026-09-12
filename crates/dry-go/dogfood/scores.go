package dogfood

// Unique bodies so dry-go self-scan stays at findings=0.

func accumulateScores(values []int) int {
	sum := 0
	for _, value := range values {
		if value > 0 {
			sum += value
		}
	}
	return sum
}
