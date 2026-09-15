def alpha_path(value, weight):
    acc = value
    if acc < 0:
        acc = 0 - acc
    product = acc * weight
    if product > 100:
        return product - 10
    return product + 1


def beta_path(amount, multiplier):
    running = amount
    if running < 0:
        running = 0 - running
    product = running * multiplier
    if product > 100:
        return product - 10
    return product + 1
