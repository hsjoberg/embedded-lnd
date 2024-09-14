{
  "targets": [{
    "target_name": "addon",
    "cflags!": [ "-fno-exceptions" ],
    "cflags_cc!": [ "-fno-exceptions" ],
    "sources": [ "addon.cc", "lnd_functions.cc" ],
    "include_dirs": [
      "<!@(node -p \"require('node-addon-api').include\")",
      ".",
      "<!(node -e \"console.log(process.env.LND_INCLUDE_DIR || '..')\")"
    ],
    "conditions": [
      ['OS=="win"', {
        "libraries": [
          "<!(node -e \"console.log(process.env.LND_LIB_DIR ? process.env.LND_LIB_DIR + '\\\\lnd.lib' : '<(module_root_dir)\\\\lnd.lib')\")"
        ]
      }],
      ['OS=="mac"', {
        "libraries": [
          "<!(node -e \"console.log(process.env.LND_LIB_DIR ? process.env.LND_LIB_DIR + '/liblnd.a' : '<(module_root_dir)/liblnd.a')\")"
        ],
        "xcode_settings": {
          "GCC_ENABLE_CPP_EXCEPTIONS": "YES"
        }
      }],
      ['OS=="linux"', {
        "libraries": [
          "<!(node -e \"console.log(process.env.LND_LIB_DIR ? process.env.LND_LIB_DIR + '/liblnd.a' : '<(module_root_dir)/liblnd.a')\")"
        ]
      }]
    ],
    'defines': [ 'NAPI_CPP_EXCEPTIONS' ]
  }]
}
