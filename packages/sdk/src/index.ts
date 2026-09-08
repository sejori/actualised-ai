// The napi-rs build script outputs the compiled node addon here.
const bindings = require('./actualised_sdk.node');

export const Company = bindings.Company;
