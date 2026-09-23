# pos-down-detector

[`nyan7natsu/pos-service`](https://github.com/nyan7natsu/pos-service) のダウン検知およびシングルトンインスタンスのためのリース管理を行うWebサーバーです．

内部で Axum を使用して，各種APIを提供しています．  
また，Discord Interaction Endpointに対応しているので，管理サーバーからコマンドで一部の操作ができます．
