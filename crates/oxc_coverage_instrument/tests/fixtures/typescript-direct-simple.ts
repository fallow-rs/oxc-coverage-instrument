interface User {
    name: string;
    age: number;
}
type Status = "active" | "pending";

function greet(user: User, status: Status): string {
    if (status === "active") {
        return `Hello, ${user.name}`;
    }
    return "Please wait";
}

const u: User = { name: "Alice", age: 30 };
console.log(greet(u, "active"));
