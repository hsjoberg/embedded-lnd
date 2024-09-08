{
  "targets": [{
    "target_name": "addon",
    "cflags!": [ "-fno-exceptions" ],
    "cflags_cc!": [ "-fno-exceptions" ],
    "sources": [ "addon.cc", "lnd_functions.cpp" ],
    "include_dirs": [
      "<!@(node -p \"require('node-addon-api').include\")",
      "."
    ],
    "libraries": ["-ldl"],
    'defines': [ 'NAPI_CPP_EXCEPTIONS' ],
    'conditions': [
      ['OS=="mac"', {
        'xcode_settings': {
          'GCC_ENABLE_CPP_EXCEPTIONS': 'YES'
        }
      }]
    ]
  }]
}
