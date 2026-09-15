def format_greeting(name, excited):
    prefix = "hello"
    if name == "":
        return prefix
    if excited:
        return prefix + " " + name + "!"
    return prefix + " " + name
