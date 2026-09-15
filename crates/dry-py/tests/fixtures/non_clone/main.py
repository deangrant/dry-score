def compute_sum(values):
    total = 0
    for value in values:
        total += value
    if total < 0:
        return 0
    return total


def compute_product(values):
    total = 1
    for value in values:
        if value == 0:
            return 0
        total *= value
    return total
