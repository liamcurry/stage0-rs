require("todomvc-common/base.css");
require("todomvc-app-css/index.css");
import("../pkg")
    .then(pkg => pkg.main())
    .catch(console.error);
