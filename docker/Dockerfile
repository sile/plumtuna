FROM debian

RUN apt update
RUN apt -y install curl python3 python3-pip git sysstat
RUN pip3 install optuna requests psycopg2-binary
RUN git clone git://github.com/sile/plumtuna.py
RUN cd plumtuna.py && pip3 install .
RUN curl -L https://github.com/sile/plumtuna/releases/download/0.0.2/plumtuna-0.0.2.linux -o /bin/plumtuna && chmod +x /bin/plumtuna

CMD /bin/bash
