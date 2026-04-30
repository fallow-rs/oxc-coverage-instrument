class Component {
  /* istanbul ignore next */
  render(x) {
    if (x) return 1;
    return 2;
  }

  update(x) {
    return x ? 1 : 2;
  }
}

const object = {
  /* istanbul ignore next */
  method(x) {
    if (x) return 1;
    return 2;
  },

  property: function (x) {
    if (x) return 1;
    return 2;
  },
};

function ignoredIf(x) {
  /* istanbul ignore if */
  if (x) return 1;
  return 2;
}

function ignoredElse(x) {
  /* istanbul ignore else */
  if (x) return 1;
  return 2;
}

new Component().update(true);
object.property(false);
ignoredIf(false);
ignoredElse(true);
