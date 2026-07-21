interface Box {
  value: number;
}

type Result = number;

function read(box: Box): Result {
  return box.value;
}

const box: Box = { value: 42 };
globalThis.typescriptResult = read(box);
