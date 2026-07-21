class Fields {
  arrow = () => 1;
  function = function () {
    return 2;
  };
  static staticFunction = function () {
    return 3;
  };
}

const fields = new Fields();
globalThis.classFieldResult = fields.arrow() + fields.function() + Fields.staticFunction();
