### `GET 200 /buses`

`https://api.montevideo.gub.uy/api/transportepublico/buses?lines=169,128,370,151,522,G,175`

```json
[
  {
    "access": "PLATAFORMA ELEVADORA",
    "busId": 29,
    "company": "COME",
    "destination": "SAYAGO",
    "eType": "buses",
    "emissions": "SIN DATOS",
    "line": "522",
    "lineVariantId": 2425,
    "location": {
      "coordinates": [-56.138577, -34.906067],
      "type": "Point"
    },
    "origin": "POCITOS",
    "special": false,
    "speed": 0,
    "subline": "POCITOS - SAYAGO",
    "thermalConfort": "SIN DATOS",
    "timestamp": "2026-03-27T11:05:59.000-03"
  },
  {
    "access": "COMÚN",
    "busId": 223,
    "company": "CUTCSA",
    "destination": "POCITOS",
    "eType": "buses",
    "emissions": "Sin datos",
    "line": "128",
    "lineVariantId": 4261,
    "location": {
      "coordinates": [-56.149384, -34.914448],
      "type": "Point"
    },
    "origin": "PASO DE LA ARENA",
    "special": false,
    "speed": 1,
    "subline": "POCITOS --- P. DE LA ARENA",
    "thermalConfort": "Sin datos",
    "timestamp": "2026-03-31T19:59:45.000-03"
  },
  {
    "access": "COMÚN",
    "busId": 745,
    "company": "CUTCSA",
    "destination": "CIUDADELA",
    "eType": "buses",
    "emissions": "SIN DATOS",
    "line": "169",
    "lineVariantId": 4637,
    "location": {
      "coordinates": [-56.201813, -34.904095],
      "type": "Point"
    },
    "origin": "MANGA",
    "special": false,
    "speed": 0,
    "subline": "CIUDADELA --- MANGA",
    "thermalConfort": "SIN DATOS",
    "timestamp": "2026-03-31T19:55:35.000-03"
  },
  {
    "access": "COMÚN",
    "busId": 761,
    "company": "CUTCSA",
    "destination": "LAS PIEDRAS",
    "eType": "buses",
    "emissions": "SIN DATOS",
    "line": "175",
    "lineVariantId": 4715,
    "location": {
      "coordinates": [-56.154243, -34.830997],
      "type": "Point"
    },
    "origin": "PALACIO LEGISLATIVO",
    "special": false,
    "speed": 15,
    "subline": "CIUDADELA --- LAS PIEDRAS",
    "thermalConfort": "SIN DATOS",
    "timestamp": "2026-03-31T19:59:46.000-03"
  },
  {
    "access": "PISO BAJO",
    "busId": 55,
    "company": "UCOT",
    "destination": "PORTONES (POR UTU CERRO)",
    "eType": "buses",
    "emissions": "SIN DATOS",
    "line": "370",
    "lineVariantId": 4859,
    "location": {
      "coordinates": [-56.083355, -34.88249],
      "type": "Point"
    },
    "origin": "PLAYA DEL CERRO",
    "special": false,
    "speed": 12,
    "subline": "PORTONES ---PLAYA DEL CERRO/DIQUE",
    "thermalConfort": "SIN DATOS",
    "timestamp": "2026-03-31T19:50:19.000-03"
  }
]
// 91 total items, showing first 5
```

### `GET /buses/busstops/{busstopId}/upcomingbuses`

`https://api.montevideo.gub.uy/api/transportepublico/buses/busstops/3663/upcomingbuses?lines=169,128,370,151,522,G,175&amountperline=3`

```json
[
  {
    "access": "COMÚN",
    "busId": 416,
    "companyName": "CUTCSA",
    "destination": "ADUANA",
    "distance": 17390,
    "emissions": "Sin datos",
    "eta": 115,
    "line": "169",
    "lineVariantId": 4001,
    "location": {
      "coordinates": [-56.18676, -34.889572],
      "type": "Point"
    },
    "origin": "TOLEDO CHICO",
    "position": 54,
    "special": false,
    "subline": "ADUANA - TOLEDO CHICO ",
    "thermalConfort": "Sin datos"
  },
  {
    "access": "COMÚN",
    "busId": 33,
    "companyName": "COETC",
    "destination": "PORTONES",
    "distance": 15063,
    "emissions": "SIN DATOS",
    "eta": 503,
    "line": "G",
    "lineVariantId": 8802,
    "location": {
      "coordinates": [-56.20371, -34.8706],
      "type": "Point"
    },
    "origin": "LA PAZ",
    "position": 44,
    "special": false,
    "subline": "LA PAZ- - PORTONES",
    "thermalConfort": "SIN DATOS"
  },
  {
    "access": "COMÚN",
    "busId": 916,
    "companyName": "CUTCSA",
    "destination": "CIUDADELA",
    "distance": 20908,
    "emissions": "Sin datos",
    "eta": 940,
    "line": "175",
    "lineVariantId": 4639,
    "location": {
      "coordinates": [-56.166317, -34.859577],
      "type": "Point"
    },
    "origin": "LAS PIEDRAS",
    "position": 71,
    "special": false,
    "subline": "CIUDADELA --- LAS PIEDRAS",
    "thermalConfort": "Sin datos"
  },
  {
    "access": "PISO BAJO",
    "busId": 604,
    "companyName": "CUTCSA",
    "destination": "18 DE JULIO Y EDUARDO ACEVEDO",
    "distance": 1823,
    "emissions": "SIN DATOS",
    "eta": 3111,
    "line": "128",
    "lineVariantId": 4336,
    "location": {
      "coordinates": [-56.26178, -34.82898],
      "type": "Point"
    },
    "origin": "PASO DE LA ARENA",
    "position": 5,
    "special": false,
    "subline": "POCITOS --- P. DE LA ARENA",
    "thermalConfort": "SIN DATOS"
  }
]
```
