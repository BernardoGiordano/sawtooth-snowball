import jinja2
import sys

N = int(sys.argv[1])
print("N",N)
with open("snowball.jinja.yaml") as f:
    template = f.read()

output = jinja2.Template(template).render(n=N)

with open("test_devmode_engine_liveness.yaml".format(N), "w") as f:
    f.write(output)
