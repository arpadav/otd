#!/bin/bash
docker build -t otd .
docker save -o otd.tar otd:latest
