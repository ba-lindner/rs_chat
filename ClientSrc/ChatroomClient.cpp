#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <functional>
#include <iostream>
#include <mutex>
#include <ostream>
#include <string>
#include <thread>
#include <boost/circular_buffer.hpp>
#include <termios.h>
#include <netinet/in.h>
#include <arpa/inet.h>

enum inputStat{
  MASSAGE,
};

std::mutex serverDataMut;
std::mutex inputDataMut;
std::mutex outputMut;

void serverRead(boost::circular_buffer<std::string> &output);
void inputRead(std::string &output, inputStat &);
void set_keypress(termios& stored_settings);
int GetSocket(const char *ip, int port);

int main(int argc, const char *argv[]){
  ///Do the login to the server:
  std::cout << "Pleas enter the ip of the server: ";
  std::string ipStr;
  std::cin >> ipStr;
  std::cout << "Pleas enter the server Port(standart is 4173): ";
  int portNum;
  std::cin >> portNum;
  std::cout << "Pleas enter your name: ";
  std::string name;
  std::cin >> name;

  int socket = ::GetSocket(ipStr.c_str(), portNum);

  ///terminal and input buffer settings
  termios stored_settings;
  set_keypress(stored_settings); 
  setvbuf(stdin, NULL, _IONBF, 0);
  
  ///setup of server communikation thread
  boost::circular_buffer<std::string> serverData(127);
  for (int i = 0; i < 127; ++i) {
    serverData.push_back(std::string(""));
  }
  std::thread serverReader(serverRead, std::ref(serverData));

  ///setup of the input henadeling thread
  std::string inputData;
  ::inputStat inputStatus;
  std::thread inputReader(inputRead, std::ref(inputData), std::ref(inputStatus));
  while (1){
    outputMut.lock();
    std::this_thread::sleep_for(std::chrono::milliseconds(3));
    printf("\033c");
    serverDataMut.lock();
    for (auto i : serverData) {
      std::cout << i << std::endl;
    }
    serverDataMut.unlock();
    
    switch (inputStatus) {
      case ::inputStat::MASSAGE:
        std::cout << "Massage from " << name << " :\n";
        break;
    }

    inputDataMut.lock();
    std::cout << inputData;
    inputDataMut.unlock();
    std::cout.flush();

  }

  exit(EXIT_SUCCESS);
}

void inputRead(std::string &output, inputStat &status){
  while (1) {
    int zwi = getc(stdin);
    std::cerr << zwi << std::endl;
    switch (zwi) {
      case '\177':
        inputDataMut.lock();
        output.pop_back();
        inputDataMut.unlock();
        break;
      case '\n':
        ///send data to the server
        std::cerr << "sending: " << output << std::endl;
        inputDataMut.lock();
        output.clear();
        inputDataMut.unlock();
        break;
      case '\033':
        break;
      default:
        inputDataMut.lock();
        output += zwi;
        inputDataMut.unlock();
        break;
    }
    outputMut.unlock();
  }
}

void serverRead(boost::circular_buffer<std::string> &output){
  while (1) {
    outputMut.unlock();
    serverDataMut.lock();
    output.push_back(std::string("test nachricht"));
    serverDataMut.unlock();
    std::this_thread::sleep_for(std::chrono::milliseconds(3000));
  }
}

//from https://www.linuxquestions.org/questions/programming-9/unbuffered-stdin-176039/
void set_keypress(termios& stored_settings) {
  // change the terminal settings to return each character as it is typed
  // (disables line-oriented buffering)
  // returns the original settings in the argument structure

  // obtain the current settings flags
  tcgetattr(0, &stored_settings);

  // copy existing setting flags
  termios new_settings = stored_settings;

  // modify flags
  // first, disable canonical mode
  // (canonical mode is the typical line-oriented input method)
  new_settings.c_lflag &= (~ICANON);
  new_settings.c_lflag &= (~ECHO); // don't echo the character

  // vtime and vmin setting interactions are complex
  // both > 0
  // Blocks until it has first new character, then tries to get a total
  // of vmin characters, but never waits more than vtime between characters.
  // Returns when have vmin characters or the wait for next character is
  // too long.
  // vtime = 0, vmin > 0
  // Blocks until vmin characters received (or a signal is received)
  // vtime > 0, vmin = 0
  // If a character is ready within vtime, it is returned immediately.
  // If no character is avalable within vtime, zero is returned.
  // both = 0
  // Documentation somewhat unclear, but apparently returns immediately
  // with all available characters up to the limit of the number
  // requested by a read(). Returns -1 if no characters are available.
  new_settings.c_cc[VTIME] = 0; // timeout (tenths of a second)
  new_settings.c_cc[VMIN] = 1; // minimum number of characters

  // apply the new settings
  tcsetattr(0, TCSANOW, &new_settings);

  // note:
  // The return value from tcsetattr() is not tested because its value
  // reflects "success" if any PART of the attributes is changed, not
  // when all the values are changed as requested (stupidity!).
  // Since the content of the termios structure may differ with
  // implementation, as may the various constants such as ICANON, I see
  // no elegant way to check if the desired actions were completed
  // successfully. Comparing byte-by-byte shows the current state is
  // NOT EQUAL to the requested state, and yet it runs, so the changes
  // were apparently made. Can not check for success/failure.
}

int GetSocket(const char *ip, int port){
  const int fd = socket(AF_INET,SOCK_STREAM,0);
  if (fd == -1){
    printf("Faild to open socket");
    exit(EXIT_FAILURE);
  }

  struct sockaddr_in add;
  add.sin_family = AF_INET;
  add.sin_port = htons(port);
  if(inet_aton(ip, &add.sin_addr) == 0){
    std::cerr << "infalid addess" << std::endl;
    exit(EXIT_FAILURE);
  }
  if(connect(fd, (const struct sockaddr*) &add, sizeof(struct sockaddr_in))){
    printf("Faild to connect socket. errno: %d", errno);
    exit(EXIT_FAILURE);
  }
  return fd;
}

/*#include <stdio.h>
#include <string.h>
#include <strstream>
#include <sys/socket.h>
#include <sys/un.h>
#include <stdlib.h>
#include <errno.h>
#include <thread>
#include <unistd.h>
#include <pthread.h>
struct a{
  int fd;
  volatile char **buf;
};

void *MyWritenText(void *socket);
void *ReadSocket(void *socket);
void Test(){
  while (1) {
  }
}

int main(int argc, const char *argv[]){
  std::stringstream serverData;
  std::thread serverReader(Test);
  std::cout << serverData.str();

  exit(EXIT_SUCCESS);
}
 
///CLI Parsing:
  if(argc != 4){
    printf("Pleas give the IP in the -ip option");
    exit(EXIT_FAILURE);
  }
  const char *IP;
  for (argv++; *argv; ++argv) {
    if(!strcmp(*argv, "-ip")){
      IP = *(++argv);
    }
  }
  printf("Hello world im a client. %d\n", getpid());*/

/*
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
void *ReadSocket(void *in) {
  struct a *input = in;
  const int maxLen = 20000 + 1;
  char *buf1 = malloc(maxLen), *buf2 = malloc(maxLen);
  int len = 0;

  char readBuf[1024];  
  enum stat{
    msg,
  };
  enum stat stat;
  for (char i = getc(input->fd); i; i = getchar(input->fd)) {
    
  }
  while (1) {

    int ret = read(input->fd, readBuf, 1023);
    readBuf[ret] = 0;
    for (char *i = readBuf; i; ++i) {
      
    }
    if(len + ret + 1024 >= maxLen){
      
    }

  }
}*/
