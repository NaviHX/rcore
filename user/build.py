import os

# base_address = 0x80400000
# step = 0x200000
base_address = 0x1000

app_id = 0
apps = os.listdir("src/bin")
apps.sort()

for app in apps:
    app = app[:app.find(".")]
    # os.environ["BASE_ADDRESS"] = "{}".format(base_address + step * app_id)
    os.environ["BASE_ADDRESS"] = "{}".format(base_address)
    os.system("cargo build --bin {} --release".format(app))
    app_id += 1
