export function formatGreeting(name: string, excited: boolean): string {
  const prefix = "hello";
  if (name === "") {
    return prefix;
  }
  if (excited) {
    return prefix + " " + name + "!";
  }
  return prefix + " " + name;
}
