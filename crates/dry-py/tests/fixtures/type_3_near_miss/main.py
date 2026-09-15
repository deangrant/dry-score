def near_left(value, weight):
    acc = value
    if acc < 0:
        acc = 0 - acc
    product = acc * weight
    if product > 100:
        return product - 10
    return product + 1


def near_right(value, weight):
    acc = value
    if acc < 0:
        acc = 0 - acc
    product = acc * weight
    if product > 100:
        return product - 10
    bonus = 2
    return product + bonus
