class Counter:
    def score_left(self, input, factor):
        total = input
        if total < 0:
            total = 0 - total
        scaled = total * factor
        if scaled > 100:
            return scaled - 10
        return scaled + 1

    def score_right(self, input, factor):
        total = input
        if total < 0:
            total = 0 - total
        scaled = total * factor
        if scaled > 100:
            return scaled - 10
        return scaled + 1
