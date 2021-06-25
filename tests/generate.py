import jinja2

N = 8

with open("snowball.jinja.yaml") as f:
    template = f.read()

output = jinja2.Template(template).render(n=N)

with open("test_devmode_engine_liveness.yaml".format(N), "w") as f:
    f.write(output)
