FROM scratch
COPY coexistgate /coexistgate
ENTRYPOINT ["/coexistgate"]
