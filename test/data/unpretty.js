class Base {
  constructor() {}

}

class MyClass extends Base {
  constructor() {
    super();
  }

  method() {
    return 1;
  }
}

let mc = new MyClass()
console . log ( `Hello World! ${mc.method()}` );
