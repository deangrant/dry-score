def with_named():
    def left(n):
        acc = n
        if acc < 0:
            acc = 0 - acc
        return acc + 1

    def right(n):
        acc = n
        if acc < 0:
            acc = 0 - acc
        return acc + 1

    return left(3) + right(4)
