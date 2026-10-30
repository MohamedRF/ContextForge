from garden import water
class Plant:
    @staticmethod
    def grow():
        def nested():
            return water()
        return nested()

def café():
    return Plant.grow()
