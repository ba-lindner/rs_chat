#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <stdlib.h>
#include <errno.h>
#include <unistd.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <pthread.h>
struct a{
  int fd;
  volatile char **buf;
};

void *MyWritenText(void *socket);
void *ReadSocket(void *socket);

int main(int argc, const char *argv[]){
  ///CLI Parsing:
  if(argc != 3){
    printf("Pleas give the IP in the -ip option");
    exit(EXIT_FAILURE);
  }
  const char *IP;
  for (argv++; *argv; ++argv) {
    if(!strcmp(*argv, "-ip")){
      IP = *(++argv);
    }
  }
  printf("Hello world im a client. %d\n", getpid());
  const int fd = socket(AF_INET,SOCK_STREAM,0);
  if (fd == -1){
    printf("Faild to open socket");
    exit(EXIT_FAILURE);
  }

  struct sockaddr_in add;
  add.sin_family = AF_INET;
  inet_aton(IP, &add.sin_addr);
  add.sin_port = 4173;
  if(connect(fd, (const struct sockaddr*) &add, sizeof(struct sockaddr_in))){
    printf("Faild to connect socket. errno: %d", errno);
    exit(EXIT_FAILURE);
  }
  printf("Pleas enter your name: ");
  char buf[1024];
  int ret;
  if(0 <= (ret = read(STDIN_FILENO, buf, 1023))){
    printf("An error ocured. Pkeas restart the program");
    exit(EXIT_FAILURE);
  }

  ///login
  buf[ret] = 0;
  int msgMaxSize;
  char *msg = malloc(msgMaxSize = strlen(buf) + sizeof("login:") - 1);
  strcpy(msg, "login\003" );
  strcat(msg, buf);
  write(fd, msg, msgMaxSize);
  if( (ret = read(fd,buf, 1023))){
    printf("Communication failed");
    exit(EXIT_FAILURE);
  }
  buf[ret] = 0;
  if(!strcmp(buf, "ack\003")){
    printf("You are now in the chetroom you can communicate now");
  }
  else if(!strncmp(buf, "err\003", sizeof("err\003") - 1)){
    printf("The Server return %d as error", atoi(buf + 4));
  }
  
  pthread_t myWritingThread, readingThread;
  struct a writeData, readData;

  writeData.fd = fd;
  volatile char ** writeBuf;
  writeData.buf = writeBuf;
  pthread_create(&myWritingThread, NULL, MyWritenText, (void*)&writeData);
  readData.fd = fd;
  volatile char ** readBuf;
  readData.buf = readBuf;
  pthread_create(&readingThread, NULL, ReadSocket, (void*)&readData);
  
  while (1) {
    printf("\e[1;1H\e[2J");
    printf("%s\nYour Massage:\n%s", *writeBuf, *readBuf);
  }

  read(fd, buf, 1024);
  printf("%s", buf);

  for (int inLen = read(STDIN_FILENO, buf, 1023); inLen; inLen = read(STDIN_FILENO, buf, 1023)) {
    buf[inLen] = 0;
    write(fd, buf, inLen + 1);
  }
}

void *MyWritenText(void *in) {
  char *buf1, *buf2;
  char *nowBuf;
  nowBuf = malloc(1024);
  nowBuf[0] = 0;
  buf2 = nowBuf;
  struct a *input = in;
  *(input->buf) = buf2;
  int bufLen = 0, bufMaxLen = 1024;
  char * msg = malloc(1024 + sizeof("post:") - 1);
  strcpy(msg, "post\003");
  for (char i = getc(STDIN_FILENO); i != EOF; i = getc(STDIN_FILENO)) {
    if(bufLen + 3 >= bufMaxLen){
      buf1 = realloc(buf1, bufMaxLen *= 2);
      strcpy(buf1, buf2);
      *(input->buf) = buf1;
      buf2 = realloc(buf2, bufMaxLen);
      msg = realloc(msg, bufMaxLen + sizeof("post:") - 1);
      strcpy(msg, "post\003");
      goto buf2start;
    }
    else{
      buf1[bufLen] = buf2[bufLen];
      buf1[++bufLen] = i;
      buf1[bufLen + 1] = 0;
      *(input->buf) = buf1;
    }
    if(i == '\n'){
      msg[sizeof("post:") - 1] = 0;
      strcat(msg, buf1);
      write(input->fd, msg, strlen(msg));
    }
buf2start:
    if( EOF != (i = getc(STDIN_FILENO))){
      buf2[bufLen] = buf1[bufLen];
      buf2[++bufLen] = i;
      buf2[bufLen + 1] = 0;
      *(input->buf) = buf2;
    }
    if(i == '\n'){
      msg[sizeof("post:") - 1] = 0;
      strcat(msg, buf1);
      write(input->fd, msg, strlen(msg));
    }
  }
  return NULL;
}
void *ReadSocket(void *) {

}
