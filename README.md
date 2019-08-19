# Thumbnail creator

Service accepts URLs of images, creates thumbnails and returns thumbnail image URLs.

To build docker image and run use:

```sh
$ docker-compose up
```

By default ```./thumbnails``` directory is mounted to container. 
To mount other directory, set ```HOST_VOLUME_PATH``` env variable:

```sh
$ HOST_VOLUME_PATH=/path/to/directory docker-compose up
```

By default binds port 8080 on host machine. To use other port on host use env ```HOST_PORT``` :

```sh
$ HOST_PORT=8081 docker-compose up
```

Also available env variables:

- ```APP_THUMBNAIL_WIDTH```   thumbnail width in px, default 100
- ```APP_THUMBNAIL_HEIGHT```  thumbnail height in px, default 100
- ```APP_THUMBNAIL_EXACT_SIZE```  true - do not preserve original image aspect ratio, default true
- ```APP_THUMBNAIL_EXTENSION``` file extension and format of created thumbnail image, default "jpg"
- ```APP_MAX_URLS_IN_SINGLE_REQ``` max amount of urls in single request, default 70

### API

resource:  ```/api/v1/thumbnail``` 
method: ```POST``` 
body: urls of images in json. Example:

```json
{
	"urls": [
		"https://picsum.photos/id/1/500/500",
		"https://picsum.photos/id/2/500/500"
	]
}
```

Example response:

```json
{
    "success": {        
        "https://picsum.photos/id/2/500/500": "http://localhost:8080/thumbnail/100x100/0b90bf6685cca9a67380fa11a1ba143c.jpg",
        "https://picsum.photos/id/1/500/500": "http://localhost:8080/thumbnail/100x100/3e01488f21a3acf704b02f57bc415c4f.jpg"        
    },
    "failed": {}
}
```


```sh
$ curl -X POST -H "Content-Type: application/json" \
>  -d '{"urls": ["https://picsum.photos/id/1/500/500","https://picsum.photos/id/2/500/500"]}' \
>  http://localhost:8080/api/v1/thumbnail
{"success":{"https://picsum.photos/id/2/500/500":"http://localhost:8080/thumbnail/100x100/0b90bf6685cca9a67380fa11a1ba143c.jpg","https://picsum.photos/id/1/500/500":"http://localhost:8080/thumbnail/100x100/3e01488f21a3acf704b02f57bc415c4f.jpg"},"failed":{}}
```

## Tests

Run tests with  Cargo:

```sh
cargo test
```
