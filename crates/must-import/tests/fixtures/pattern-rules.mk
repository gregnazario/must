%.o: %.c
	$(CC) -c $< -o $@

app: main.o util.o
	$(CC) -o $@ $^
