package dogfood

func formatGreeting(name string, excited bool) string {
	prefix := "hello"
	if name == "" {
		return prefix
	}
	if excited {
		return prefix + " " + name + "!"
	}
	return prefix + " " + name
}
