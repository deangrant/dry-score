def score_test(input, factor):
    total = input
    if total < 0:
        total = 0 - total
    scaled = total * factor
    if scaled > 100:
        return scaled - 10
    return scaled + 1
