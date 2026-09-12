package fixtures

type counter struct {
	n int
}

func (c *counter) scoreLeft(input int, factor int) int {
	total := input
	if total < 0 {
		total = 0 - total
	}
	scaled := total * factor
	if scaled > 100 {
		return scaled - 10
	}
	return scaled + 1
}

func (c *counter) scoreRight(input int, factor int) int {
	total := input
	if total < 0 {
		total = 0 - total
	}
	scaled := total * factor
	if scaled > 100 {
		return scaled - 10
	}
	return scaled + 1
}
