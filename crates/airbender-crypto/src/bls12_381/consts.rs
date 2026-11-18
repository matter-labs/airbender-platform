#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    test
)))]
pub const G2_BY_TAU_POINT:
    <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Affine =
    crate::bls12_381::curves::g2::G2Affine {
        x: crate::bls12_381::fields::Fq2 {
            c0: crate::ark_ff::fields::models::Fp(
                crate::BigInt([
                    6998771983072852473,
                    11736241389176950350,
                    14652389186963586383,
                    7123021877941670904,
                    207427363641627917,
                    1666061032901291221,
                ]),
                core::marker::PhantomData,
            ),
            c1: crate::ark_ff::fields::models::Fp(
                crate::BigInt([
                    1270972800850449493,
                    331328462692285148,
                    9602917463918608193,
                    2816806383447892978,
                    8933573566397811232,
                    215261465954158607,
                ]),
                core::marker::PhantomData,
            ),
        },
        y: crate::bls12_381::fields::Fq2 {
            c0: crate::ark_ff::fields::models::Fp(
                crate::BigInt([
                    12255148049650361111,
                    16300459039673357879,
                    7278512065901627776,
                    15013916996328221833,
                    6959599066670318708,
                    1753751357774418949,
                ]),
                core::marker::PhantomData,
            ),
            c1: crate::ark_ff::fields::models::Fp(
                crate::BigInt([
                    6097766243631356938,
                    3657144287806647550,
                    7252852235594748032,
                    6043526089682840990,
                    694068262573112211,
                    1355366081521641917,
                ]),
                core::marker::PhantomData,
            ),
        },
        infinity: false,
    };

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    test
))]
pub const G2_BY_TAU_POINT:
    <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Affine =
    crate::bls12_381::curves::g2::G2Affine {
        x: crate::bls12_381::fields::Fq2 {
            c0: crate::ark_ff_delegation::Fp(
                crate::BigInt([
                    15222373064398286084,
                    13305997496817878699,
                    6179074517294182750,
                    14794871321375031765,
                    2834697192260086091,
                    387745707543054929,
                    0,
                    0,
                ]),
                core::marker::PhantomData,
            ),
            c1: crate::ark_ff_delegation::Fp(
                crate::BigInt([
                    802106297986366494,
                    7763332301576374198,
                    16078281631408652708,
                    4142264898103746401,
                    12005984959834078047,
                    248731809877450469,
                    0,
                    0,
                ]),
                core::marker::PhantomData,
            ),
        },
        y: crate::bls12_381::fields::Fq2 {
            c0: crate::ark_ff_delegation::Fp(
                crate::BigInt([
                    4900293511062467887,
                    17213741567581943225,
                    16312230343184456439,
                    4417609035285159901,
                    8724769964152345554,
                    1569984678681432578,
                    0,
                    0,
                ]),
                core::marker::PhantomData,
            ),
            c1: crate::ark_ff_delegation::Fp(
                crate::BigInt([
                    10357602823164305765,
                    17761333828174651100,
                    14619682016189758143,
                    14389745726652808402,
                    3537342951673246453,
                    1861810530228151377,
                    0,
                    0,
                ]),
                core::marker::PhantomData,
            ),
        },
        infinity: false,
    };

// println!("pub const PREPARED_G2_GENERATOR: <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Prepared = crate::bls12_381::curves::G2PreparedNoAlloc {{");
// println!("    ell_coeffs: [");
// for i in 0..prepared_g2_generator.ell_coeffs.len() {
//     println!("        (");
//     println!("            crate::ark_ff::fields::models::Fp2 {{");
//     println!("                c0: crate::ark_ff::fields::models::Fp(");
//     println!("                    crate::BigInt({:?}),", prepared_g2_generator.ell_coeffs[i].0.c0.0.0);
//     println!("                    core::marker::PhantomData");
//     println!("                ),");
//     println!("                c1: crate::ark_ff::fields::models::Fp(");
//     println!("                    crate::BigInt({:?}),", prepared_g2_generator.ell_coeffs[i].0.c1.0.0);
//     println!("                    core::marker::PhantomData");
//     println!("                ),");
//     println!("            }},");
//
//     println!("            crate::ark_ff::fields::models::Fp2 {{");
//     println!("                c0: crate::ark_ff::fields::models::Fp(");
//     println!("                    crate::BigInt({:?}),", prepared_g2_generator.ell_coeffs[i].1.c0.0.0);
//     println!("                    core::marker::PhantomData");
//     println!("                ),");
//     println!("                c1: crate::ark_ff::fields::models::Fp(");
//     println!("                    crate::BigInt({:?}),", prepared_g2_generator.ell_coeffs[i].1.c1.0.0);
//     println!("                    core::marker::PhantomData");
//     println!("                ),");
//     println!("            }},");
//
//     println!("            crate::ark_ff::fields::models::Fp2 {{");
//     println!("                c0: crate::ark_ff::fields::models::Fp(");
//     println!("                    crate::BigInt({:?}),", prepared_g2_generator.ell_coeffs[i].2.c0.0.0);
//     println!("                    core::marker::PhantomData");
//     println!("                ),");
//     println!("                c1: crate::ark_ff::fields::models::Fp(");
//     println!("                    crate::BigInt({:?}),", prepared_g2_generator.ell_coeffs[i].2.c1.0.0);
//     println!("                    core::marker::PhantomData");
//     println!("                ),");
//     println!("            }},");
//     println!("        ),");
// }
// println!("    ],");
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    test
)))]
pub const PREPARED_G2_GENERATOR:
    <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Prepared =
    crate::bls12_381::curves::G2PreparedNoAlloc {
        ell_coeffs: [
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5141805939031916900,
                            14881501738404200130,
                            5676796723958628022,
                            12444886031927648209,
                            12625577537049268217,
                            1648088581216593497,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13479113584676927735,
                            14085014821599008390,
                            12897808079030738240,
                            10758929507152881398,
                            16000768153542152626,
                            1172245526740373679,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2148348102093263616,
                            12232197882281708926,
                            11330363351339265390,
                            8919790901940522406,
                            17524282994943615806,
                            496043075549758450,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            406374380327561821,
                            16222300308001049590,
                            2744191801523148582,
                            9384378502465456106,
                            6088477103101489105,
                            1478173219842328727,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2384913353902199319,
                            7760278026988209995,
                            10908782382662037359,
                            17048340870424330239,
                            4284142373730869649,
                            1799494596850233552,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6808605746120772081,
                            17102333730072771654,
                            6473213958624321251,
                            8206845943439019082,
                            9001841424686257916,
                            263896703973961396,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            410815106749295669,
                            14671497359075212928,
                            5247007732706898941,
                            17995312000949087238,
                            9318564848658720568,
                            830601423731554979,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5581652307577543765,
                            6048112805369680744,
                            10863925005663083759,
                            13723116258784921171,
                            16395951836908287261,
                            576183509239513289,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12359103411754323777,
                            4484518591789401692,
                            7553773553318136256,
                            9286007000674216720,
                            3828416673463588070,
                            1125060269472609951,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16270530695471883024,
                            7398690485593453415,
                            7661592259938731084,
                            6115274743727668626,
                            11646088452095310487,
                            1172296607029761726,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13799898119383583275,
                            14835071743299103237,
                            2637313866028438562,
                            18274788277677870627,
                            13617768339621759057,
                            1000507734120925728,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17140326549953569073,
                            6405224081483886159,
                            4267674644342821653,
                            1752649571364847099,
                            17969610602415877411,
                            1346486438763963099,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10372035707327665855,
                            13033699652831669377,
                            18001351867719217419,
                            17683642914756429388,
                            9556503388952039857,
                            1329751260307086052,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2344862993485028348,
                            8828144198172398125,
                            15447134648735934574,
                            1255270699924723893,
                            4886950282422369309,
                            381483219941628951,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11860626640653029501,
                            10904903791216459764,
                            13503337534485290667,
                            346033757805938283,
                            10106952548686363100,
                            1513842329147353970,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17736603614208695589,
                            7622430165596287250,
                            10019522101254057211,
                            4957042600265288097,
                            8842339425901560598,
                            1070474381731750456,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3126979498979956446,
                            6683277335328149417,
                            15253880732281146257,
                            15099174266347432605,
                            17950830287545815649,
                            218127290741943543,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2868616128842733870,
                            8017633674659729200,
                            6433577224475778073,
                            586972730864327558,
                            7642330394223174199,
                            1383486225402290655,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18198021688502592557,
                            13511431506245271930,
                            10721587455469480135,
                            7759712521116984140,
                            4136567657482394076,
                            1322321864369983077,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5385833590826609721,
                            1997327134082967041,
                            16887896265967856133,
                            4670107666134914874,
                            8408948779659614002,
                            966249059862278969,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4238264172116927440,
                            8585427175208453787,
                            13391259632752185786,
                            2243605050137253147,
                            12543049565771064759,
                            1744764348013310000,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10362084567046626758,
                            10816154860909344542,
                            10573511294357547907,
                            140128829695140438,
                            13150599586971606962,
                            1132304614921786427,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2154321054217735380,
                            8325838573332693936,
                            9961626471649444564,
                            9464856130848169875,
                            14569577991616642249,
                            1187070117018910587,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12768121148781055196,
                            18245949438107747183,
                            9780133711340233987,
                            14985049703492470765,
                            7812515833497248970,
                            995044817595131952,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            381452297110706448,
                            14888063619316195299,
                            5903322847657577559,
                            4192955803278209460,
                            17451506546072907376,
                            146610250215610338,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5427213429271106499,
                            4333616755713581723,
                            3271300846951325985,
                            18251281755642440121,
                            11279254981659276170,
                            729466300166508568,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15793577716612855564,
                            4668995723578274815,
                            4384187013721788325,
                            5005115314512922510,
                            16580401117618392401,
                            1061089863156435375,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10938483931507615313,
                            1819128771204318317,
                            1378166881235931358,
                            13169265358183292197,
                            8495579604450832241,
                            1658457572327554200,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5606543612283953239,
                            18041223158520881442,
                            13479600114443068848,
                            5715782193307494167,
                            11021015089636409126,
                            216773321964843711,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2929533507575320551,
                            3443042620097897503,
                            14715630991325653192,
                            14609224186819678494,
                            14472777980914477678,
                            1241469894408677550,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8573336919322429465,
                            17992052812250631772,
                            8731453543706499896,
                            10664025860267270772,
                            230615772046556522,
                            266962162194865915,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17761807615271640858,
                            11489295081018607087,
                            10788634817114068653,
                            9639855738763174930,
                            13638905777067884774,
                            996032760442672471,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            417385912334655687,
                            16112303583556661074,
                            4458661395407831437,
                            9526451496016245577,
                            3386449271250125514,
                            730353849140560753,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9000093142487517991,
                            14134083959924237055,
                            18046389804750210448,
                            2943907734641935785,
                            11874227498753799708,
                            467415405859937517,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6399931434183934664,
                            2348668329180844113,
                            6545865515938198761,
                            3174039629982378188,
                            8156147867999962162,
                            265247106625245813,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3685955571729247289,
                            13154819674840828100,
                            2572300054611671044,
                            15295760164169411393,
                            4605338285028612462,
                            777648684134450039,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10847380501819118332,
                            9997176788977361883,
                            9145374535949399277,
                            2988305573081111221,
                            16532364284655040017,
                            22369720389082729,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2128613708163288559,
                            3984312897046484124,
                            440690462454569345,
                            4447100961186936601,
                            17011428506723439829,
                            464381823536933373,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14624486359648699748,
                            9996471425625772906,
                            7959195681278883241,
                            13187373556334020786,
                            16378006220526491063,
                            1102676257961916954,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10565173302830080041,
                            14557534785903372419,
                            15050302666547062955,
                            7876976764100481255,
                            4104086943124558954,
                            561283378587482515,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            709562408010554375,
                            4267992058656169535,
                            11687232113456093746,
                            11896453304250293406,
                            16082573870442512457,
                            1474877609868443671,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5000373916021640150,
                            8070258887298854009,
                            4784603921999825690,
                            6187343649437885286,
                            18164391822004564300,
                            101412407463769652,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13471431044201719510,
                            13652983820932105600,
                            16391697356344427978,
                            1909258935459139345,
                            4675889529846639014,
                            403289222787366231,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9211307257257923428,
                            1458284542262832952,
                            9868036486545772136,
                            4852373548617371498,
                            352514220866497285,
                            1364013305721814584,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8005481355175942977,
                            18325163562340905436,
                            5390710820200297131,
                            15110003324963209796,
                            10874791894834869540,
                            1392509089925744797,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7471236998666819885,
                            9992485353963224673,
                            5586811759391298758,
                            1425063232120675711,
                            10061366176295910900,
                            322974810213550416,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4777449248121411850,
                            295102903503592931,
                            13035937688357174264,
                            1734138769696788538,
                            2291149507807446168,
                            1849870684541128156,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16975918388064884856,
                            12834867506673749379,
                            648523850022296783,
                            18069076335058758742,
                            9876590207854679329,
                            1346654090519127856,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15747728109020361321,
                            13725681923363316770,
                            5324450391747943978,
                            774517115391587398,
                            15069895026598897351,
                            912503434719372401,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10149825125588160762,
                            11820756380776900023,
                            15159495854088463628,
                            2632153691577698988,
                            8726849390783566255,
                            1418039180389332657,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10949388138415984785,
                            1653345621151405861,
                            6989611558301588196,
                            7094158119030897521,
                            8092598974865067730,
                            344903474317610242,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13179427330210479777,
                            12627360700998004214,
                            16078285943569855890,
                            11503046501803791217,
                            12020735899017724573,
                            1587835784911466986,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4736315383562732347,
                            4857279745231940348,
                            12084525333989199454,
                            11935005340671850738,
                            5292246436264611071,
                            1464813745154178522,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9671749090904758079,
                            4011208751311086456,
                            17258777116205478546,
                            3988062033120029806,
                            7555416602408608721,
                            1230456088786320969,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4359473842744791483,
                            15229196376600208182,
                            16390865079379463113,
                            17304456992172995643,
                            8241645050374294242,
                            1048598515423429339,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5030196255765617673,
                            907619662631850037,
                            4908147951400445547,
                            4565498463706034311,
                            3222701189937984820,
                            1137793701347911568,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18266295415946367821,
                            9218337022030772249,
                            4573512429414012619,
                            13221532010045438499,
                            5438782004406746723,
                            1173984545771228727,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            592313367490451090,
                            6144941370159173573,
                            4415719859404102956,
                            12295351402307904606,
                            14201585298553426051,
                            1792589111712859574,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1520984947060149855,
                            4670730037180607416,
                            5364080033783607774,
                            11030375954854580964,
                            6606500366184623640,
                            1015849247070053398,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1860475962392426848,
                            6414572385931489470,
                            944547912718495232,
                            6207857425764014254,
                            16082638886377558425,
                            435828459834202715,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12415664568316923969,
                            14153586859668198792,
                            8313325181049576803,
                            11856236819688804952,
                            8670376532791302504,
                            882355792588751782,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16708868432318978244,
                            5890490433612928589,
                            18409737640877154902,
                            122052811551250142,
                            12310803993302720147,
                            251506331851000332,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9109251818875808493,
                            8434405903908160911,
                            425240150822053835,
                            9399371041275642928,
                            6287726593966843142,
                            1081706261152752478,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7807978281705858889,
                            13014064301541624918,
                            10752549970233376499,
                            7038868237437625253,
                            8518172012238343172,
                            472844660464331547,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5428618434409641282,
                            731074514690730256,
                            4909671977543008161,
                            10685029362744112834,
                            13579270631866498815,
                            670612544934760906,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15779443715945418536,
                            17234857151081093038,
                            3340207207509495517,
                            14906529178208012763,
                            17348922171258971904,
                            160543278375823719,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            572813010612831175,
                            17229932213218921994,
                            153412800968486396,
                            8382115482058901058,
                            13947916826235297912,
                            1060434287659930064,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2909189869389827583,
                            16283414275911366367,
                            16367387944566926439,
                            9066777010217601610,
                            12074685396270575418,
                            740216266393022223,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2221509424448989182,
                            8029152828048580472,
                            15231187909079953032,
                            16774319742105172202,
                            6628826656573200829,
                            1595226693061121342,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            887879947476000210,
                            14916108503088875124,
                            4255290164931162535,
                            17187108110984313117,
                            12416729357880053981,
                            506145233301733206,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6688122711399903377,
                            956534379015613198,
                            2306064226352684985,
                            14943216876664937687,
                            9922665993481137055,
                            449473707222814818,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7366760327299120827,
                            6723698819112066476,
                            10691768784174752242,
                            14405362052410758714,
                            3186794622910528358,
                            1628256393874806324,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5510538203572665789,
                            11026297340378949532,
                            14224245128751044848,
                            15089766391051904644,
                            10215127899568402056,
                            926540898138293552,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16523419256453396264,
                            7548530122574515251,
                            12254353138466430556,
                            8191411044395027797,
                            4584362460891999351,
                            1248660522713978812,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14379946145017925561,
                            3327578865202539495,
                            15983014440733985013,
                            16918401789285250319,
                            6804175225703239091,
                            629825587369830914,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5238084445534838637,
                            6349281509122043694,
                            15029679403126853381,
                            3543305929084475098,
                            8043862043604620304,
                            1774530426378903388,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8705402993275412290,
                            10309994477918659991,
                            10194982214192083307,
                            9832470544682507482,
                            13235909624056037536,
                            1213280542750553218,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9858695170949036472,
                            8190792129305206992,
                            18316361779769977680,
                            7065501875819745623,
                            12512614800124303252,
                            424780995124882112,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17091230753738073939,
                            5742407283561017981,
                            11206869815880854568,
                            5039463592612019006,
                            5706493686670480942,
                            1869829432548034141,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9918794650019348421,
                            13764666769916554807,
                            10148964309945121407,
                            6669075856076202274,
                            8950652948676731827,
                            859093905425356226,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5801944926310406828,
                            16606365636472891299,
                            11510564985002054245,
                            8618221901061351118,
                            3423654271244297912,
                            1254790916538181892,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14351542720162538378,
                            13077311130522818611,
                            3235161037234002053,
                            7772420373540310536,
                            17687285537178996666,
                            1112662618840671728,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9034883416908472689,
                            5293397624124428877,
                            10246982669704808706,
                            6986607116401213771,
                            18044558202738306921,
                            86588148847963213,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5370662249630643241,
                            8474350153879675377,
                            11632312534461028740,
                            7535231338191187612,
                            6586752577274218807,
                            1264764077468486862,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8280979498285331584,
                            17019750325519637000,
                            7076069628962101063,
                            7563539879491222615,
                            6172127473873583107,
                            1854881132180286239,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2071835565841741777,
                            1956669178919454095,
                            13011612914992615232,
                            9082336034965990027,
                            7660270005804839215,
                            1377361658900982072,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14358037576172456719,
                            4870327049814028108,
                            10919817122507308475,
                            16861828022847424174,
                            6937130395560086657,
                            1706623480645447844,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15667541763677614986,
                            8885513261058484931,
                            5248016105658358130,
                            6461060351251084201,
                            279338902814023444,
                            1859033859549368815,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18184577908890743160,
                            18356792231154436931,
                            10133245481394331698,
                            15644737709399520633,
                            9330036923167937703,
                            1147182018750432947,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10394453543147136379,
                            8191410394774562711,
                            165994681954033257,
                            11805522380584290897,
                            7981452714904861724,
                            1823514220346625232,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6246772424223103206,
                            13535707299370117047,
                            13571982035269134435,
                            12465208524416615863,
                            6166422329514466200,
                            538700431202549478,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14764571985258794292,
                            10322901757714296314,
                            277697261275397439,
                            13500953434633153854,
                            15469838882117236241,
                            1123610510755063835,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12029239395035277093,
                            11002805357522421531,
                            8999246049032739379,
                            13120287455103617564,
                            13472516720644507470,
                            323554578848932618,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5809710973960547215,
                            13458204376064628589,
                            2487973437886396009,
                            9176715112333432629,
                            7469121850432659151,
                            1801907694856932304,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11059972478455275483,
                            9255163636708247029,
                            16604769607715476492,
                            3656985343498043274,
                            2115634676761475788,
                            1251991146855816309,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17657908188424387426,
                            11073798568900799444,
                            4455764421800215857,
                            5801378529356094867,
                            15616431480564134346,
                            1796496794381845238,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17519954294993772358,
                            9688472811803864561,
                            4772609518940248259,
                            1486397643694454637,
                            15796702127462147232,
                            745061313639134949,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6492332148656391120,
                            11892647445697960374,
                            4956028036621043758,
                            15873228217171977725,
                            7088847408232978988,
                            884458387948228392,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15266144299652080222,
                            4435468330079935905,
                            3183565181247742018,
                            6437035485349062759,
                            17688765152509296440,
                            1868715196972502551,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5741957419475912963,
                            8713011334077104954,
                            511029427401626408,
                            8519202553144504147,
                            7704058049079094113,
                            626976432701717203,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14685537232609441349,
                            12817215031100449805,
                            7062803415061165613,
                            13061056474912282785,
                            16292628649392942774,
                            1726411621393573495,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7342034101471681091,
                            5981831773055475131,
                            15751998565050527759,
                            6827405799516296521,
                            9563311848952801517,
                            64756942261636680,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9312245213474993364,
                            763253257073167467,
                            2533148859694067490,
                            1557031086110277901,
                            8508168896787537761,
                            77324790895599152,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11234318394460893824,
                            13080925536205801339,
                            2974438010892781680,
                            16798346758236335495,
                            853019663129303221,
                            1553116686383780998,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10672509030427770486,
                            3411471258178267916,
                            15147115224899113286,
                            6509466393481970020,
                            789267814514693398,
                            759336200565465981,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18044316525074213694,
                            10583433656152181769,
                            1369264306974089385,
                            4568949137998859190,
                            8676791248189369854,
                            599810496658723989,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6592321761334491193,
                            17371940768472854432,
                            18368764698137791841,
                            4573106669022277271,
                            17136105106659820995,
                            1552408082806665468,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6679852690623383792,
                            7238344873834000234,
                            17655879693118300498,
                            14744078909744608320,
                            9873788307834513723,
                            216056756403160835,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8737197971574139456,
                            2470794839196658321,
                            10167394307573068236,
                            4990015158953624993,
                            3762261760425381855,
                            1117493865737123846,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2809336211689955115,
                            461364399510825826,
                            13512322279040218911,
                            4747323802367068145,
                            17008695436363516654,
                            1752048705458801231,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14061116903649502484,
                            16775564879532606396,
                            7762197473669358824,
                            7279166923317389734,
                            8458737303118396614,
                            8541471356393900,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5370719352741660522,
                            14190399276457775395,
                            11856188007549148553,
                            4290771095642050714,
                            5685314401789769717,
                            236157616473756757,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8101700174229828763,
                            14734600490653211235,
                            8320672592581831775,
                            9563374028059576675,
                            10324661543306959069,
                            1492437861443819879,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10865908099406512244,
                            15866458552322176026,
                            13549264719367926426,
                            4076155156194156988,
                            805455541784346072,
                            1448155476839248250,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9428156655815602008,
                            4108443916605113695,
                            5336647495022411943,
                            11345695112801438703,
                            6695223989892280346,
                            612625508919689927,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11119920249334032857,
                            1554550422566802323,
                            14975928531586102399,
                            7684854428208335140,
                            1185344066296825033,
                            1838127699743186716,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13180385423122124352,
                            1480508394386712760,
                            7616313138304958378,
                            10703476620031135522,
                            9291484443676454627,
                            1296044813488827620,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1292122395223995643,
                            10547441950937890577,
                            6288558001390788576,
                            3266695088792339778,
                            16340091016898498587,
                            305104804562104473,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12614657435499982406,
                            7321760421299539600,
                            8887870714814981984,
                            17308926768123915024,
                            10907982983608707333,
                            254782569586417921,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13443662230275057808,
                            7283338715646491712,
                            8762652993539391275,
                            9568118572053462216,
                            421911577020099018,
                            651333865719837370,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13558817906561336108,
                            5851162910471130994,
                            4839547741286309451,
                            14916751504369247656,
                            1551420211443359333,
                            151100226727178033,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12072854520021699150,
                            12163738072090558741,
                            8790616344897720272,
                            17628405401840194959,
                            17914738184224504104,
                            1808478666771880843,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14972613684404784505,
                            10854582706172394085,
                            17054500206331327498,
                            12676207388239629423,
                            13441910894169340413,
                            580331324581542656,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4556215054000136015,
                            10993047464222978897,
                            5917325015059164450,
                            11431004719854752055,
                            6777047625237478073,
                            1594739766495549815,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8368524145996765235,
                            16792128449470148343,
                            7451501716641093279,
                            15314826531557276399,
                            12289973506047041927,
                            1736559317069223212,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10334865954750154795,
                            12276715663254174420,
                            6192749220561928606,
                            12959596323639225997,
                            15985863425125289966,
                            1822630449876104576,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11562904017718506633,
                            6579608218515679898,
                            1836041044185674882,
                            1764269830347154390,
                            2220099293563482223,
                            338555480421158471,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10100356761369142420,
                            10424655188420335441,
                            5734489069780548435,
                            11804531177499496491,
                            15540257393125983282,
                            540426617410161771,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3903728819734581724,
                            6017989364983983723,
                            4347149887966132417,
                            15724076786717717654,
                            13422800188273302224,
                            75114195302985353,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8927408070343603731,
                            4744055380721541335,
                            17356141953351775670,
                            220262449755162239,
                            8762371885422412323,
                            875041224433176916,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7805613614301416893,
                            8211924119434414988,
                            4780969413207420175,
                            6054971174804680918,
                            11789086836721105818,
                            364984441896091125,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13707114789309096817,
                            6983770212939376625,
                            4652999218495815702,
                            15390124120475979572,
                            11425897909266781607,
                            1815736988543237201,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            756429358325308477,
                            659541189319221444,
                            9076774734801098537,
                            7423351133737467281,
                            8587430796292973973,
                            354344686039546896,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8409542993120211214,
                            4381373496129436439,
                            16433219173975623284,
                            18413920590028294169,
                            8153936644811655429,
                            1129864039932514084,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13104989724667548822,
                            16376820145402192197,
                            10318058046867619731,
                            347271512513951709,
                            5037624056388832095,
                            766809507996547074,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13673623915494396761,
                            11178100410220631593,
                            4169679698331960833,
                            932618932010414866,
                            8093871536244890828,
                            422732936255767862,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2248031022531194288,
                            13182615268354766205,
                            17159159755157149353,
                            17466312037965417492,
                            3947940995603806712,
                            1805221247387915355,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11650022333419710480,
                            3821434330496576576,
                            11067122868503029041,
                            4710264436379844964,
                            4608125004352504360,
                            377939507088559664,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            323427229854884849,
                            15059129573007958674,
                            10108807310890432816,
                            2192584389865522553,
                            8603319949293960431,
                            1211995643565977375,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6617523029171881322,
                            11383322448186641350,
                            7559578974833833008,
                            3205173471494285020,
                            14392352251207536437,
                            1216518386488752117,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13974112595347915982,
                            18220925998206931233,
                            5819973887253711430,
                            10731293668836791307,
                            14884656678543918401,
                            553607838101055965,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18342847706747975145,
                            17503336598929234156,
                            9082996789852941719,
                            12254582692588866283,
                            2652816587616976547,
                            1651037606221886301,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3806695740296916359,
                            3621254523957269984,
                            12563782648420016231,
                            10854775247040384554,
                            12923280103678034507,
                            334333064748926871,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7986793279595160847,
                            3115107781589608662,
                            5549751392645077837,
                            16637642137988243682,
                            10778592284957097351,
                            1112601254317530668,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14276054406963632957,
                            5334816257639817066,
                            16918079679538189141,
                            1256889063742606781,
                            4429197869623683683,
                            1052564087951581473,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6587815683562939879,
                            17061663948376645289,
                            11987879735554540077,
                            14687742048052059379,
                            15871762594691957824,
                            1400545276768732750,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2857182733695150298,
                            14492502368509374315,
                            6394066382421140984,
                            4873260557352586907,
                            11745621581369190709,
                            101070833293163892,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1830781412772034822,
                            11065996574206904394,
                            443407900315515204,
                            9864221378907827448,
                            928707473418445932,
                            789312371568169664,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1631293021391196217,
                            9348919760617510194,
                            5157507494237347818,
                            3412204342022241830,
                            9929962472745352202,
                            250781686293569324,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8725641782772531504,
                            6655947609054648794,
                            16333363275704951817,
                            12246989682647476912,
                            14833870257003335928,
                            1360627535322727843,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16479828748373544357,
                            288126515347333515,
                            8466738550426328621,
                            9581686375219113341,
                            132931039437676687,
                            1093670044843279481,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13247169693856882953,
                            13854495156863289204,
                            14916431986517228713,
                            14891433230462954539,
                            4630658022816049516,
                            7946960084262839,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3968197022474644210,
                            13691032437123974025,
                            3257576533498966575,
                            17424029346920503662,
                            1219026027825904483,
                            432630522521653764,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18415896655570881926,
                            15471807232790398093,
                            11105822436321302454,
                            1714823536627667804,
                            13286914906765947930,
                            1671434534756521091,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1948713421501294402,
                            3666068177285254931,
                            3302136597595848638,
                            5774412565041263881,
                            11858949347321653231,
                            445026608623954694,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5191139556204333805,
                            9820054992783904697,
                            2586059096705926519,
                            15740220772239480604,
                            2912029280360817240,
                            935213602419325709,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1657715447292834868,
                            2965196754196176708,
                            3507960075236678865,
                            14257122211122657297,
                            16195261890988014160,
                            1359438125688526049,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12813818712711389308,
                            10204629540719373260,
                            12017802221556548665,
                            16454940640503084174,
                            13481633468586520142,
                            682815932022050192,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10936361258559783826,
                            9568753568013478204,
                            5069569775606198918,
                            14636241003936320403,
                            2766342499692823628,
                            290192532328908336,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6065710111562110300,
                            2669228301371161303,
                            8791675820899482243,
                            14422205176293018020,
                            13603583659291202708,
                            1120871229977348719,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17992903148930215522,
                            8699796732563272905,
                            10856881822414778484,
                            4897040836582574597,
                            1364711228382736183,
                            593678163703515224,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3065481212355142345,
                            16980518859180403147,
                            11449642751002013335,
                            15493100692066621770,
                            816310608108964677,
                            157154334540632315,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11827439129624723879,
                            12960531134143634868,
                            81100493982484384,
                            3881019493167631986,
                            14505070854566559906,
                            1009142801207247198,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8806400921730365629,
                            4604747154207809521,
                            7212994567852887195,
                            13789244228662861640,
                            5618793575574262131,
                            854116858602998744,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3659507673664194883,
                            5029714648683569775,
                            2930917609321627441,
                            47952116182632615,
                            6307287339046222113,
                            1318306840534101160,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15612234697892050000,
                            17724285545119211529,
                            702481085194978052,
                            14919206601691285639,
                            12661058390066341103,
                            1133561129594690505,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6825797417508498353,
                            7601601940903371561,
                            18086777943531459348,
                            10915505851005145273,
                            8295310039502550522,
                            208526734251212012,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12809080936152416425,
                            3363446487535372208,
                            1593669525614429479,
                            18366712507472896252,
                            11294551264162408956,
                            746850206714260505,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11896272653453487718,
                            12262211153702241855,
                            15900558098585718890,
                            3969340192269706987,
                            18116125173439016592,
                            638156942425780148,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17819551186075410805,
                            1831090028451772986,
                            16446626650644612008,
                            6877510800182892895,
                            6107112253482296948,
                            949924039180853543,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10289249185598119887,
                            583806682878801538,
                            15243531005351046092,
                            4191309864744162614,
                            12134446094112313096,
                            1253555380725307195,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14208315524873981697,
                            15726946123006866023,
                            2200825923806428221,
                            8928071804522453729,
                            13691968997549726781,
                            188718982116483912,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2817695138244938028,
                            17942739338936562790,
                            4078976393999431199,
                            11066789245987752446,
                            8777372617925054103,
                            565981311492096061,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10447940287494510448,
                            12907759181700104722,
                            15177912804949353464,
                            6032826097982879706,
                            16231974647869412816,
                            724404737633441835,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            273530767975359809,
                            640960343280265344,
                            3206660265780635369,
                            10627544832607645001,
                            7924183912727038157,
                            1031020725916350848,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3916993637981712528,
                            4336206702819842879,
                            4752647567496437063,
                            12411432589668429002,
                            3797020205905694526,
                            575240292666795475,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17906823193119094511,
                            14885792574463768817,
                            1055186485892938401,
                            14891180091066733776,
                            4312935767833134159,
                            1259051711244060747,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15602357124582241437,
                            14680316575002632180,
                            8438191597647786843,
                            12517236504392829175,
                            4040605163119008780,
                            1021972437210527156,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10219967886937106821,
                            12963678402262470926,
                            12756161081647870463,
                            10134451477496366268,
                            12215456402301222627,
                            1470621567471326550,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            154401509468976557,
                            17584684348770715131,
                            7543898025024632959,
                            18099797128522450533,
                            9337011953917962307,
                            1490382734303727912,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4458984523275779495,
                            15613803599795879682,
                            9503372989488317926,
                            17421070354463841259,
                            2041345037914509133,
                            269118574277721126,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6344353256113505955,
                            3622962565745525717,
                            5606919033076814384,
                            2696263065597548087,
                            11639795274690362850,
                            1161562882011478630,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4412980905618251017,
                            15537880068613179206,
                            369513134308123982,
                            4564375012623591815,
                            14433272751693670981,
                            1580844509527173911,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2762808031102482190,
                            16150872093625360011,
                            1331101168933797021,
                            6590314366537614994,
                            12691269149061414756,
                            982831943855863273,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5376934285612957961,
                            16876177625654590350,
                            11973740203103571480,
                            7060895647881444494,
                            13153722607967103553,
                            571864000812697046,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7246820349047722207,
                            4867979251579790772,
                            9021375131075207640,
                            12970184790434113824,
                            2553229341212193248,
                            832490965875941417,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9274201758346276486,
                            3788672265227661435,
                            1855235771841319911,
                            13957589706885013341,
                            12829857355013233963,
                            1058395368379912176,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17925207873879493365,
                            3719844925213384705,
                            15566239488935555669,
                            15697338598725600748,
                            1056501701568394255,
                            1323913789671533448,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16693810223377953954,
                            8992979349074895166,
                            6230675067474713795,
                            10250757017916809653,
                            11589010918628184101,
                            1153472976462911857,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15238598884229569208,
                            10803660612249481426,
                            9218191100816051507,
                            11340704888317265653,
                            17361440015823407755,
                            1720441625680538654,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5441093851164674869,
                            1800963049840822269,
                            451736645629520554,
                            3476225213562820998,
                            7449469664746243912,
                            1598790789065845332,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17644793259946941268,
                            15090693303518547252,
                            14904814944164261900,
                            3388360116089914674,
                            1215944320002867760,
                            871622829572904421,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3188436387852423243,
                            14993756172975508805,
                            18130395671291577498,
                            5774043178238259953,
                            15336268316706529154,
                            260671535524585275,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1909807180595145393,
                            6221295890287774404,
                            8841757198385160332,
                            1473027161152451999,
                            11073410034004334514,
                            1840888323905894817,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2803714541482159866,
                            9337216940067663510,
                            13691847009863426938,
                            15285614033467414566,
                            17722449335888191406,
                            862483000481405739,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12049355098210974057,
                            1326104058858507219,
                            3708382304498040404,
                            10303147766405142052,
                            17121523616779111396,
                            972180771722297409,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10375654766556953472,
                            2487054310364910604,
                            7877220313556469568,
                            17935294906001081987,
                            782136806585869042,
                            174783090445631703,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12157421733708710033,
                            14144237238048503245,
                            7518540404899857534,
                            529844606823728640,
                            1237451933311754628,
                            900738385531471495,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            447499281329623709,
                            14479152397836594719,
                            7461142143690023183,
                            9700092266766488397,
                            17544865510199315743,
                            1375287079732080800,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9913628416916474269,
                            8390439649295024054,
                            1270962387064159511,
                            8692614407920393918,
                            5659200583125508553,
                            389876469424149391,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4538883002515732662,
                            11001466556661716354,
                            6501875823728744525,
                            5193067771817008024,
                            6548593115572784407,
                            114550064927459080,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3971065204850673297,
                            8863508345468882221,
                            9302332606512685031,
                            570550130368381465,
                            16439498351906305598,
                            1631908396184674497,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2135920517443507378,
                            13825023977572313533,
                            16853422899225116596,
                            9300531534110891403,
                            8761160236662485095,
                            526931700001302156,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15445271443545116080,
                            4890507702034957078,
                            15159637508638863599,
                            2044740287104007297,
                            6909379997148374399,
                            1747560486961801133,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16395155563036064542,
                            16118493481324474667,
                            10598595526989639487,
                            14108600693041977859,
                            12123471075139739434,
                            22761574491135085,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10681254512753394319,
                            4136080076831107785,
                            3959828045334609588,
                            3812814985574108015,
                            13467601519655943212,
                            1395264649173308771,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            574913706145012235,
                            8436060835033739005,
                            13596647401322912938,
                            9323610082217101238,
                            18270504072132355342,
                            392088878688125872,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9931059746520888646,
                            7332998347018661869,
                            5803276161270726044,
                            3433170595431564336,
                            8723478007701976591,
                            264220272549826459,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5285241078847621520,
                            11962514263092929059,
                            2202143964754956553,
                            5691997987866788542,
                            15798436842476235793,
                            902353860156111516,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12624428720858408614,
                            8765762813120459871,
                            11880377705517380615,
                            1550404360018567261,
                            11898602958633805843,
                            1609733032427676325,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6373465907492637063,
                            17811666536366150282,
                            2460704908256228608,
                            11755791054413605069,
                            17916415863562502748,
                            1037079854987328209,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14919978742464217634,
                            1741257500454590564,
                            15633257012491385946,
                            1068912165064278574,
                            13704360611864819076,
                            213127566294379095,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14877620808295892171,
                            5904048013482912894,
                            12225876173656649719,
                            12788976918807240667,
                            8467813405004346992,
                            1163130203889086667,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14877247712565120860,
                            11235942375535900845,
                            8180257424437776314,
                            5574880854503671831,
                            2954625920323569521,
                            1836333400700562338,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4687407716328619684,
                            2085255772484791463,
                            15463630759112731974,
                            10908085673071208893,
                            11042459575812361094,
                            206085496107063010,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9447127269985211075,
                            5273059548574530004,
                            17046020599249028614,
                            13457955683260923947,
                            8617667592455421701,
                            1024744763154092460,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11728810668201589824,
                            13860043067999472965,
                            1213434209717171378,
                            11824196852601883395,
                            6823859597543923657,
                            1507393704166490576,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10921490262709603640,
                            9837397298248190298,
                            1745433799620417038,
                            1301121920924094449,
                            7207585168146122369,
                            257300796630018037,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1061348541736409971,
                            16656949433737689916,
                            6631090845639787437,
                            15522185084087965540,
                            6017061975726002327,
                            123635952079412331,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7740004307559021012,
                            8561083038510029334,
                            1147539059770274511,
                            10504475757094058646,
                            298927325257596852,
                            1716575662989644286,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16648074241240248648,
                            18274684562189032639,
                            17360352817804970713,
                            1059627640875741436,
                            6897991225438707457,
                            629363636812793092,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15873473250236395643,
                            12977645007356703813,
                            8208924490408901447,
                            4599318807417937516,
                            483548642174731811,
                            222163419728575055,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6158796921437644583,
                            8370789166527684801,
                            5760511341802896891,
                            17945126048013647151,
                            6450997361693031811,
                            813115530765987639,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2273493884542555372,
                            14333200620786586253,
                            10559541673100513443,
                            7553625226323259795,
                            10777770590235906211,
                            978754887841112378,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8256227538920604942,
                            6111022756881818707,
                            13053122837052267120,
                            651897312859112357,
                            3016802473491248619,
                            1049407615558035226,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14364693709097481213,
                            12340868517265015944,
                            8736889476635081411,
                            7917880176286895781,
                            13370846970018036261,
                            1740088274742243246,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            524907353785450792,
                            15695601711423124186,
                            16911140830048785722,
                            11155824942809572925,
                            1770167768104118045,
                            777116731273387352,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10310937194804331620,
                            7637704793335670030,
                            7316173518283538746,
                            13366444768516630043,
                            1310491413285186700,
                            1337943973262041692,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14421048613253225665,
                            9797353975377286039,
                            7214508657673197879,
                            9146269721913933729,
                            4710560144112653236,
                            1673516520094669593,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1497622130892396390,
                            1685174150335999689,
                            17352312188836824522,
                            10139936009166615853,
                            17350898555052595477,
                            1419273397312173847,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16861046793701440639,
                            15250682641580774803,
                            555730900539281901,
                            12692740996580732515,
                            5395965899205941366,
                            1808528827748692474,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3098127189548887015,
                            15994738185042809244,
                            12411425234484117159,
                            6542382283625911324,
                            4654548441214649860,
                            431820555911675094,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2063071015836874970,
                            6840062203494022182,
                            4246906740621860236,
                            10347110875184985451,
                            3418282834817861917,
                            792403127842874800,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12411294235993556822,
                            6095942616613941634,
                            1264723007403367433,
                            17217693954228208161,
                            12837288654241068219,
                            1054814062074977847,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16345770028904744153,
                            635125758526069578,
                            13439582931103153126,
                            1020746358833587565,
                            904416854996290702,
                            1501061386345380706,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1757920279692427649,
                            18250564919363320036,
                            8788373708794817081,
                            2614680759282236424,
                            9267340562510926376,
                            1689983745832315252,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13117188184349176794,
                            9213746859351061973,
                            6206940721877177228,
                            10058174399688150253,
                            1490027588809854309,
                            111794144929683848,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3140692817466575112,
                            10106745724048169351,
                            2998507736300218301,
                            3725327402338198404,
                            9497342645347040977,
                            1659526413135649020,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16808624171886853667,
                            16823473022172203203,
                            2636185418996595896,
                            9682239813936974363,
                            9385139519878354739,
                            1439707913581902285,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10903121819971190926,
                            6301654292330854238,
                            622005511920086765,
                            86988489766571632,
                            7931113254436311922,
                            1840637847662929388,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5382989087246186575,
                            12611284546852156697,
                            5519007439087895825,
                            17267655992774560163,
                            15751083557506950874,
                            881687229034211373,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17470869782826080067,
                            14529183271117580885,
                            4753254732840383124,
                            4324084504937291015,
                            2552103603585105930,
                            1532636300247273429,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4303969820917004885,
                            16021322405070684967,
                            801514668775943850,
                            387090479516039467,
                            14861161134438853533,
                            1849811583389758609,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6098376282457688380,
                            2994986269629813457,
                            15989528436836051918,
                            8293271197016415590,
                            9632661798666043861,
                            1026559911101782200,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11986159923773006299,
                            1760375557396412509,
                            1096017653307098561,
                            3760674156213476675,
                            12067894833165901186,
                            1668810088621756976,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2042101157000966398,
                            7462055591349297228,
                            12104856671158231381,
                            9488827218597976755,
                            6136719543784469427,
                            436892495147116682,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13271770293435464535,
                            16197563408818897353,
                            9816930109073256953,
                            729214006287386557,
                            11927401393758584942,
                            49270738778169830,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7468886689336311320,
                            4759013954267918844,
                            2346172033566943771,
                            1192658728962029589,
                            14580388074944279118,
                            25590512253797962,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16400469989521551329,
                            14110364153711685630,
                            7517941015120787605,
                            7672599722420857808,
                            5185510060443190203,
                            942295098188018604,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13600955351968491944,
                            1176097789172245068,
                            12690409046523452044,
                            6398083700373400129,
                            5273897916138114699,
                            655719147052956322,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3963257258200804747,
                            2775797544556973959,
                            2940298404702965828,
                            2412744012885299363,
                            15545173526659350678,
                            388949182738911812,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3668785576722942204,
                            7249294278380608852,
                            13759030843631123525,
                            12215165387169092548,
                            5478351532888416875,
                            1547723398824710037,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4106699163198478467,
                            16839371468142614396,
                            16301215183118884666,
                            15169264832648796323,
                            15707777618212581667,
                            969960710948051225,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14598655048953364033,
                            8615324336260651737,
                            15970289678594653060,
                            2870734462625256187,
                            13647993669315812604,
                            1761232944315904675,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17585718292781595853,
                            14991205606377529575,
                            15074272135415351370,
                            13375869852001983627,
                            11054569396805776167,
                            927411424184104512,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7136560337966585341,
                            17745790229256045319,
                            5740144305982986561,
                            5417019123785848959,
                            18366577115079994571,
                            16685878888735315,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11495994509590059745,
                            5136784020063164036,
                            18078166988629502114,
                            16059086812426354324,
                            2209784122419294348,
                            1872818377118103742,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14437536084306211044,
                            2740119505754636304,
                            14887480180026324418,
                            6297788622344952083,
                            1014510258580108648,
                            491488379993155018,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8152577241533669276,
                            16536715003179716814,
                            5013364591910946032,
                            10476769608483189650,
                            16195222532199643034,
                            1774833168141260467,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13164266012407861771,
                            13889378600737292392,
                            13534801411086474702,
                            16493245232092274994,
                            18294328952236143749,
                            582973946380157112,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5998430215038144200,
                            13321010093839282367,
                            6355097140409156300,
                            17699122333059865088,
                            723381955488197603,
                            1752335823727171331,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8178378178964942425,
                            9690440548689074571,
                            1595239067887263702,
                            18246567274138096780,
                            12749575382483691112,
                            1117708454142771352,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6826943486004856546,
                            18252232290117612309,
                            16613874782269471735,
                            10995498202085839269,
                            9839203529012425954,
                            360662255776773701,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2511215022943472746,
                            2151498992163238866,
                            16874250069783055797,
                            8999429806181255120,
                            14973179934442845045,
                            1525815828660616547,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9317391341099404465,
                            16297310759480927000,
                            6062528274131147824,
                            3135883026756803893,
                            10550902392195492265,
                            1119024169417391583,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            311751782623208441,
                            15308013993573478419,
                            1447985556696587802,
                            8907269195813833246,
                            8926204583111173860,
                            385935389515190331,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7854360764642732589,
                            2197239552644963643,
                            6052113846242702709,
                            15731492239268983402,
                            13435933342835215032,
                            1355419882633272473,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17940706519477320747,
                            14604417898788785723,
                            83616261297132160,
                            5142722845448990123,
                            13157345363188253197,
                            987094135952052072,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            817576745842838727,
                            3752924033538816527,
                            11867010305307261464,
                            16239711530954178881,
                            12171047215837153488,
                            1439936650697082151,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1747255690104780089,
                            12331885010613715056,
                            18221767152818872114,
                            15942183720261926491,
                            12958660637375673564,
                            129036590065882848,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5969052953054689877,
                            16804734598565554010,
                            7346486627198660091,
                            13132376355489037157,
                            14621567297425687225,
                            1538165698240311584,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14045106345594092327,
                            2344720966773406875,
                            15704452785615335497,
                            9586786496211365777,
                            14301365767236171066,
                            1491370388304233653,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            112847569975435618,
                            2855011004390726126,
                            16666425836734398553,
                            7427401460247264422,
                            8930997958747968245,
                            941036243695178874,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6854176806441209952,
                            12452846921780398163,
                            10534590437174292092,
                            4157156765560989367,
                            12560340964386033937,
                            735499109515154957,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12868905878357315350,
                            5720457962818245394,
                            8499563938777709836,
                            1281605558793637176,
                            9052129869434703579,
                            932775640995920332,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15519245782387265988,
                            8965045589516494184,
                            14808629855723674572,
                            3477560561115016769,
                            6561027564782843847,
                            513536291561502076,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            358529453782366833,
                            8028322383692735528,
                            9267675569568673861,
                            13842451876326330593,
                            12434109116672330952,
                            995331262320207668,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2760256781738382532,
                            15543786468979870096,
                            1832074120111867574,
                            14871376433774997473,
                            5722239134705046853,
                            1184623726580659227,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2620758877365803179,
                            2146849051218906659,
                            16819755666756490750,
                            2671946394905233218,
                            1050142790719909726,
                            624074784988256019,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6566189454337295375,
                            12247294398119054834,
                            10722793470383301364,
                            5905519004925859975,
                            4517572920402335205,
                            509862736104351308,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15569853168920263456,
                            361197763499607806,
                            4003306921178996318,
                            17484618443118177221,
                            17462072783479530815,
                            420536359715603289,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17404024440860530247,
                            2559496277438558450,
                            15275341193758303572,
                            9916662702013170149,
                            14705928314281568400,
                            1502711175441676760,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18299144478966412892,
                            9262574033600178742,
                            3611043133597558110,
                            13896889307585154195,
                            7137001299317170158,
                            222940116695315308,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17636175901343449131,
                            10122383361345456211,
                            3523569282000241994,
                            10711521739350062468,
                            11571092932783985328,
                            991865777959814710,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9211934819990870596,
                            746245598164526808,
                            15713659156674722721,
                            7493013283684933399,
                            5973216279782849303,
                            1319690663858459127,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13207795863090796735,
                            8805612809295103223,
                            4784689056009220385,
                            4779394889098154256,
                            9440700709172463218,
                            1296452206193991108,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17715278236299607254,
                            6736439616463458006,
                            11038702887485942095,
                            15867804669036769526,
                            15634881396005817993,
                            489723018123298278,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1132162753583317856,
                            10277749087604186671,
                            2156841408148863095,
                            14046619671361840627,
                            14527320179355494350,
                            1521728716123160304,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14880215038721983766,
                            7387017523869429623,
                            17525999497280475373,
                            2380213343767944389,
                            9298862318316667707,
                            202047976778748515,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17495609747118830892,
                            10624484726923714660,
                            928001122155035043,
                            9180480136517984389,
                            2010016949815381476,
                            1809111527722906898,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1276934049014168426,
                            17276259311312628937,
                            4454076464824832889,
                            16301021201864034716,
                            8554975839098852766,
                            1458254812293818352,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7370589103868337910,
                            7875428885664578646,
                            17821887070850190125,
                            1726558993439482913,
                            17607731644779182843,
                            58735883930758391,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4547179141298766498,
                            7928969349288893386,
                            9404663890689026548,
                            11442273783165918777,
                            6241828040065135576,
                            1449357837746612826,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4830200267341305732,
                            13326111200092022226,
                            7450166445394390911,
                            11780673972623080478,
                            11446275300389411534,
                            827588453927130689,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13775940385084320299,
                            1972197458532403414,
                            11293923482485216782,
                            6460425606821847702,
                            1499788463996260506,
                            1620338708077809457,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6414268186586043316,
                            12478734243317707800,
                            2127837062602571476,
                            6251272720315557475,
                            6958058782770939414,
                            705146260919294060,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7387940570081342206,
                            15864631906126525410,
                            5237313169925674691,
                            10566319873514363291,
                            12887222303354613523,
                            819857065025035973,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15849157766345815947,
                            17004442624020788924,
                            15157123601131088152,
                            2424187017549975695,
                            6238805223338829593,
                            48107028212652075,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17990156117996354303,
                            7614302510487808932,
                            15352171224978273855,
                            3886432970865616940,
                            17288872436394837629,
                            370332122754624701,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1039109280292018695,
                            16583278523664793843,
                            9150267651887264280,
                            8663817771282245154,
                            4411283383737984247,
                            913945778067562110,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            305239749439892347,
                            800896027095572031,
                            6665037694304341,
                            11615696819982717509,
                            4338274716423768850,
                            1819962152045105013,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1806020103061386019,
                            7512847780014857209,
                            17094397007385037263,
                            316976600296578903,
                            401824747693126931,
                            548665527085722626,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3268685494497437767,
                            2080768970020944170,
                            2201735027910066163,
                            15458776692425830533,
                            2855112192250237611,
                            1513862753357530327,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5765969437519781038,
                            7920064697548272513,
                            11577488360486961158,
                            1839701258924740355,
                            8355871817728826171,
                            213266834882512443,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13476999144548679199,
                            9223629678314753198,
                            15433658711165222706,
                            3274602718410116298,
                            2599836751271234598,
                            1572096896368899673,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10423572297317691196,
                            2410573942084075013,
                            5950428235090737163,
                            16220464213449650592,
                            15317281349909137657,
                            1023319033027183011,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16540418349536687587,
                            4714793885263410257,
                            1183427607985861498,
                            3927432755476133183,
                            1162459966883173807,
                            352524813769969331,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4292274949408048322,
                            18192569304892911421,
                            9012356623930242566,
                            5739784363489062221,
                            4761260094004811729,
                            662718641348462616,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12086106591189901952,
                            11772063701557307576,
                            127860271682619651,
                            8895234449845459576,
                            12659121583485797654,
                            893236344384702902,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            999628549523765250,
                            17585768012291868471,
                            16179386747410357622,
                            10260780326409122862,
                            17342318601981071424,
                            879489197116237949,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17135650875019297117,
                            8973866350611335408,
                            2817772143296052760,
                            8311290227566691458,
                            5194443964302441476,
                            232054768591486061,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12197605605619767530,
                            9167861191842116778,
                            13289550063991233166,
                            7337565126402696897,
                            3996690786507181786,
                            147743495143234507,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6131990600103396770,
                            7537007034668572580,
                            3113705088722491092,
                            16217494464704995384,
                            4867344927662487314,
                            822308441637851790,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5562887427295293882,
                            8932777604121834338,
                            13206052999825282881,
                            2029397130855713889,
                            5266328360773418202,
                            1571041739963428279,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6766948076524794878,
                            5524163308829394687,
                            10947099391491609549,
                            11917569319004034198,
                            16323566958142703707,
                            1115135720175226146,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16146354649106487098,
                            256378285812368071,
                            9492767563206114645,
                            17249583047652680260,
                            17922820346193058995,
                            144282512688821677,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1810277271446360408,
                            11850796728294152714,
                            57851788112027534,
                            2109087389872636618,
                            9675898346022620635,
                            1084632926635979412,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12756063847267106679,
                            3321147515220005874,
                            15543113047199253995,
                            3344629895161813177,
                            16379494060978304053,
                            494857061855567884,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12034513295991798277,
                            13360782581322425245,
                            1725302584472980609,
                            5860344302874327127,
                            585907958187382909,
                            399850908026468668,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6365626450037446919,
                            13899131710773498567,
                            12460495842367859121,
                            16170929542485821506,
                            208255995547163999,
                            1788709529122513090,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11608572617590639088,
                            1490066462143332180,
                            18104854906992827701,
                            9920434731054710041,
                            211765713996340813,
                            1553339364874916386,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9672666264253487202,
                            10336654795350669889,
                            449862483373446854,
                            8801793815254199265,
                            1432193855307402981,
                            1150842027610094260,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16630684536941166445,
                            94594638205119849,
                            6999348924643195115,
                            18325706953800674390,
                            7488829312874030254,
                            1277679837592286485,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2043409913398544163,
                            8799985458975684188,
                            11810346102690400206,
                            7287894179620391849,
                            8310030298617803903,
                            72032388626634878,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1281687477320199555,
                            7563076564081146202,
                            12967015875403459476,
                            14624806766159224062,
                            17996965057478167606,
                            252697541808936833,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13793957312069841765,
                            9280360942893255165,
                            4205627493290145795,
                            17041753664664484143,
                            292481827112153331,
                            1625837563384118732,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9352821740625689875,
                            6308883394178894773,
                            3845557142717715027,
                            2629762210321359507,
                            17751017925156195727,
                            584390040153245303,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6686080398875968317,
                            3061215826479322264,
                            3497897865939501121,
                            18281203246973482576,
                            17927392922522140979,
                            210210068325921232,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16607078842599531378,
                            9504606871982910509,
                            3444663148800932338,
                            13100534280090774060,
                            2271611617452010488,
                            330988216624333302,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15201230330916224449,
                            17352678142014047801,
                            2904416965439394519,
                            13623439688205147735,
                            15906870593512811607,
                            1219691388678138207,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7840239581072302125,
                            2230979975670043222,
                            2803040505698332898,
                            12053299447726410370,
                            9133499313447891576,
                            1792952726533648441,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10887159154609486766,
                            3586570219945765498,
                            16987857331424166578,
                            17227117211665904563,
                            2641362075356050937,
                            1107000584903662835,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2913822763351979027,
                            16605086612048089786,
                            3073014559686911583,
                            3859287879108764710,
                            8979599193271739338,
                            717888751414334418,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10679990968047840039,
                            10565037045739067909,
                            3068293805934655617,
                            11345115244705867351,
                            819555464480452960,
                            1359092330283576500,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14986598304387577492,
                            1475342186073787009,
                            10690740891793407718,
                            16999563299462835243,
                            4555879779713993726,
                            1804953060349525147,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14718054382508842189,
                            3739789684741379050,
                            9034135208882203942,
                            1626888902243251070,
                            15015377529971837185,
                            1774026940931530558,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16283842137357890083,
                            17042287683087456568,
                            9283959198140871618,
                            5116811499550729674,
                            527971125133441524,
                            1195755169769435304,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2263768794863831072,
                            6053804060596607861,
                            9304430160953566875,
                            10800763975545038529,
                            4491057833324637918,
                            429009324512387952,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4474279464499229184,
                            12996303765150012187,
                            4641054898176728295,
                            6223252192051069941,
                            18203596112407286527,
                            375775551469839185,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8672165050655637275,
                            11592407134014470035,
                            12429878132507516313,
                            7584016740753175759,
                            14004723765673647138,
                            652321248730214643,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6411316145341220134,
                            5097903652746372194,
                            3084321987603443066,
                            291263048266979464,
                            13948504323175578887,
                            745571255052464834,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13766498570618749733,
                            18086239932265263044,
                            6267759234237396905,
                            7524577637333901477,
                            18426300225099406723,
                            869027842015464503,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7915505991108860376,
                            14744184946864966602,
                            7841743679530089832,
                            6342118044023938028,
                            16924058188347940268,
                            921722487859844584,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18432711285445907477,
                            4799959705089732945,
                            388658094467292109,
                            13469196397244777115,
                            8027960956661936508,
                            1856146042798927224,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9412562462505690188,
                            8821023218336407305,
                            17692993600679959856,
                            16005771856109996480,
                            16508814953731959950,
                            156651092503265291,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3784443491987350905,
                            12821878762449544181,
                            15570828798018678235,
                            17369955329858634994,
                            3933669309037306365,
                            246493789403749385,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16488715180078991174,
                            15204639058814585608,
                            17282981194905586800,
                            15264446678800350625,
                            3984886682381078986,
                            1671127256012160488,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17590970556421112644,
                            17176240063847474694,
                            10088027845441536104,
                            6389168435284829065,
                            1591653901588710987,
                            1149030296320687731,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6079225404636231740,
                            11577093260786694347,
                            15113701608174770334,
                            2514789638500123591,
                            12818253947778157652,
                            973877333193733730,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            122455346643796234,
                            8815451192692205966,
                            11588754695599824954,
                            16375258168570638365,
                            10573408258322703739,
                            1309894564644849748,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15507485838998532052,
                            12273896981046538735,
                            9564646143178881986,
                            9799403820679217277,
                            18407515367424986612,
                            1458100371033925231,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6741837792526180340,
                            4741093931866041713,
                            11759041003213664145,
                            645125858661861983,
                            1040599062942109222,
                            1094091021380223034,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1997113849244717466,
                            629467846506294903,
                            8233285395657023714,
                            17967730458865455833,
                            5557919433109374072,
                            53265650382370398,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            111447671463676950,
                            4524009178586750554,
                            6241552241237694589,
                            15516227124308709619,
                            1634653178781246845,
                            308225627496460724,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11425290477616553189,
                            8987143399419740775,
                            7242024229149284738,
                            9341911953611946022,
                            13875120599558885654,
                            451265879496184204,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11256787455103266093,
                            1868940201283063058,
                            1806970943669599128,
                            11534684408870469118,
                            11050307924636247838,
                            1747625844835066119,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            55418155768619976,
                            2034658883971349892,
                            18274630924121538016,
                            1665817702737637819,
                            1836525310264334874,
                            1087170227886131971,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16334506042230835085,
                            3206237476817598184,
                            12402834128857567616,
                            8339689657317707135,
                            6029051804515072820,
                            1456846465263568157,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16766707407501672142,
                            1887530354787759486,
                            16790922829511246380,
                            11009431664827237384,
                            13153188622730201576,
                            1838374361829005667,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4910545072837641961,
                            13403460400440102710,
                            5273352206218781216,
                            676697953860982177,
                            10825617927840762982,
                            285154055105445443,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11746400297651516832,
                            10979763446858812125,
                            183692602325584858,
                            8498320789284697326,
                            462797629762387415,
                            576405248258865140,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            777911193742572296,
                            7647513902921402961,
                            6067446540972771385,
                            10790916516077108642,
                            17675182722691613012,
                            1751686398229996040,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6405068608950491031,
                            14140957542216444822,
                            2309070038411189322,
                            356329552979368518,
                            10765941306340344398,
                            116831036886342988,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9024698106098702623,
                            12296815156425022583,
                            9784564377917285395,
                            1404052146912132637,
                            14541191129419624393,
                            1226504296184078727,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9027381342836181758,
                            9138486834763091990,
                            222415840784199390,
                            14489635729577468753,
                            13929972455917488456,
                            1786529869997611434,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7403194024900695680,
                            5637255599482643285,
                            5375494383863040234,
                            224644731955307803,
                            2873214725430145840,
                            104229465812347909,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3439363775230345751,
                            15638073239631313515,
                            9492418863020281360,
                            8131969094489376602,
                            5720770235757025622,
                            483724292851919853,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16699187852082723843,
                            8693120683959256561,
                            16145520580880843176,
                            15514255596695428931,
                            7841666762005833637,
                            420652186032170947,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14335513727543274208,
                            16646213806585277802,
                            12925600038488807498,
                            2605225084328483855,
                            14665570212563299912,
                            754384477496729360,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6694419024724290957,
                            3871170988293794892,
                            15732304184831754055,
                            9738283805219832287,
                            7675180625074030238,
                            480847214481638638,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4529064992016325751,
                            15671417487523988332,
                            7508561555645614607,
                            18434413552791868384,
                            5809087897244316529,
                            1676080464407039490,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14253176987565140087,
                            8432973936757336868,
                            12536150827545251950,
                            15984093800740611917,
                            330194173108652513,
                            720863017847274140,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2504083676569153968,
                            15832083804081244200,
                            16877291253304412991,
                            3123117264403828128,
                            18348442583875952540,
                            10672200961417660,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5638240506016061059,
                            1583690574560387490,
                            17099322707903237119,
                            4602001013727840879,
                            11805945594538167489,
                            769932996100306776,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13389054708145450145,
                            14824985038812446465,
                            17048423736687968055,
                            8814579705583440961,
                            5035209798269158033,
                            749603826618513244,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3328160815093979948,
                            6237421512227118148,
                            5454867442369564245,
                            8360606765311549714,
                            506884495890557126,
                            1337844504046619073,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11246208992146705102,
                            7491507820739201488,
                            8226502557152353642,
                            8242655725648707598,
                            2563614515138514716,
                            1276767182461265012,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4769926818852819626,
                            12326920208947329191,
                            7133440403278935560,
                            8217617849285520661,
                            16049604563650375359,
                            711475378941333023,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16849766076317370723,
                            10565333070263476703,
                            3115741580833081075,
                            8928575428429098701,
                            83956234334344128,
                            926347158376151918,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10143750035226921107,
                            11175567661712278494,
                            3216221237041612947,
                            12620844773409697702,
                            10707220252741199768,
                            1623526040161124106,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14447879463473006980,
                            5619619168231237359,
                            17851493197815746321,
                            2472919125584316304,
                            12932681200314166169,
                            32006887797470998,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6733468596582028491,
                            4938403331373994423,
                            2495886566350463086,
                            4866913292325304512,
                            17833122831271837090,
                            20747965259049769,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12123540221335456053,
                            17481962647007948085,
                            17714790377634962640,
                            14772495719257247834,
                            18145394591723123802,
                            743560198215921556,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1497908276767573553,
                            12116779752273264858,
                            2590346265392281813,
                            2332181715169792445,
                            17092360546369766522,
                            1785077242918424407,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7952464444820094318,
                            13994002917888001241,
                            15706352139461318104,
                            9628004532299569283,
                            8518954296988900559,
                            852566979184224097,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7441466901095813604,
                            18132532258406101602,
                            18424073174602445664,
                            10981127994613099451,
                            8652506872459021220,
                            1468509449400294373,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5657596159808172880,
                            13526374509927555697,
                            9098455594626067101,
                            8158374417330316391,
                            17542823587561952014,
                            1304079557315137315,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2947031150797965961,
                            7168775351093980616,
                            5402014775348167763,
                            15061123952343550529,
                            18424749141952968,
                            1371189299929536049,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9281444850959927837,
                            1614617486747393747,
                            11241920734328704958,
                            2097073444320196800,
                            17161985834411206816,
                            1087148532604594683,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4447772820428775845,
                            12960992846973464060,
                            16293514322279992379,
                            12893219225638396788,
                            7781299197431263334,
                            449901882076709110,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14907262724371019648,
                            2800189046514750300,
                            1193812845074755789,
                            17554157846753588042,
                            11314840220597096532,
                            1498650060415121036,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14577735060881878484,
                            14619568849073463350,
                            1431448144111700444,
                            14528704181850128793,
                            14501293334336711131,
                            1612428065818573674,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14062135068179357635,
                            15712636924265402003,
                            14349265695217949070,
                            9810173535875805004,
                            4832166941298165361,
                            371959303338356286,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            529620776199412663,
                            11279048013297394977,
                            7438985769468435174,
                            8418833291556253638,
                            12638750430770189374,
                            782893910818976383,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3425819771147962005,
                            9716494799582968327,
                            16725193404816337991,
                            17445947387384886143,
                            10172343138072657791,
                            1282876810846454564,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11164718869065048732,
                            17084654245035325868,
                            6328765895618868472,
                            4486022148862603580,
                            8280153517244863665,
                            888195886403406946,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18324010328125972459,
                            10571572315927437740,
                            4014634975442021698,
                            11429030044824256915,
                            5271595227066846426,
                            1674840201622272053,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16223747653705116452,
                            4719335504099331335,
                            12299381611509085527,
                            5371163119486524817,
                            15960986105789375268,
                            1057686279051569806,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3231538934033646755,
                            5344125384418809110,
                            11846526592348732580,
                            2299306435200444082,
                            3692252847535720845,
                            575369555574054056,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4973247319762195912,
                            17368940014671360540,
                            14839003924064767409,
                            3245510716526580765,
                            3436906785389213579,
                            844931647848380362,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15165477183340963026,
                            16997843586210142388,
                            4445003245359517877,
                            10927166688015442507,
                            7520430330020049843,
                            1420012853112495986,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17593407237472846452,
                            7139603939882108134,
                            3757323610542394666,
                            4472807864131808150,
                            3386962978897274091,
                            1383468498082191913,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1444325385217431665,
                            10835112227600171299,
                            8404917474308882742,
                            6750769422842145945,
                            13441473813250589699,
                            924170415944226904,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5548361823936492326,
                            14178292541188102100,
                            5559690584309841024,
                            991693879832300891,
                            8159539515805530943,
                            441534492458820411,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12926075692359746707,
                            2076456620501910467,
                            11812451018956222528,
                            5858709688062104928,
                            15645714274888629921,
                            1083342478455859008,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15181447811226672777,
                            15314942397772641642,
                            10472930132225485603,
                            1443381424108076475,
                            6075609957057421500,
                            1525570165854124564,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14187469699259158108,
                            1543752628374803254,
                            5434623545917424877,
                            11549575584320809126,
                            5114744709675570251,
                            1705988908992398926,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4574256762420014476,
                            3109595645412534119,
                            18207041838432318466,
                            13843871776053619491,
                            13206492776194894486,
                            1386537781114910309,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
        ],
        infinity: false,
    };

// QuasiUART::new().write_fmt(format_args!("pub const PREPARED_G2_GENERATOR: <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Prepared = crate::bls12_381::curves::G2PreparedNoAlloc {{\n")).unwrap();
// QuasiUART::new().write_fmt(format_args!("    ell_coeffs: [\n")).unwrap();
// for i in 0..prepared_g2_generator.ell_coeffs.len() {
//     QuasiUART::new().write_fmt(format_args!("        (\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("            crate::ark_ff::fields::models::Fp2 {{\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                c0: crate::ark_ff_delegation::Fp(\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    crate::BigInt({:?}),\n", prepared_g2_generator.ell_coeffs[i].0.c0.0.0)).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    core::marker::PhantomData\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                ),\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                c1: crate::ark_ff_delegation::Fp(\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    crate::BigInt({:?}),\n", prepared_g2_generator.ell_coeffs[i].0.c1.0.0)).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    core::marker::PhantomData\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                ),\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("            }},\n")).unwrap();
//
//     QuasiUART::new().write_fmt(format_args!("            crate::ark_ff::fields::models::Fp2 {{\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                c0: crate::ark_ff_delegation::Fp(\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    crate::BigInt({:?}),\n", prepared_g2_generator.ell_coeffs[i].1.c0.0.0)).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    core::marker::PhantomData\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                ),\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                c1: crate::ark_ff_delegation::Fp(\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    crate::BigInt({:?}),\n", prepared_g2_generator.ell_coeffs[i].1.c1.0.0)).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    core::marker::PhantomData\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                ),\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("            }},\n")).unwrap();
//
//     QuasiUART::new().write_fmt(format_args!("            crate::ark_ff::fields::models::Fp2 {{\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                c0: crate::ark_ff_delegation::Fp(\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    crate::BigInt({:?}),\n", prepared_g2_generator.ell_coeffs[i].2.c0.0.0)).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    core::marker::PhantomData\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                ),\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                c1: crate::ark_ff_delegation::Fp(\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    crate::BigInt({:?}),\n", prepared_g2_generator.ell_coeffs[i].2.c1.0.0)).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                    core::marker::PhantomData\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("                ),\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("            }},\n")).unwrap();
//     QuasiUART::new().write_fmt(format_args!("        ),\n")).unwrap();
// }
// QuasiUART::new().write_fmt(format_args!("    ],\n")).unwrap();
// QuasiUART::new().write_fmt(format_args!("    infinity: {:?},\n", prepared_g2_generator.infinity)).unwrap();
// QuasiUART::new().write_fmt(format_args!("}};\n")).unwrap();
#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    test
))]
pub const PREPARED_G2_GENERATOR:
    <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Prepared =
    crate::bls12_381::curves::G2PreparedNoAlloc {
        ell_coeffs: [
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3052494208186889246,
                            711327711884005890,
                            6566687770021815863,
                            5689367844038720698,
                            7839048598077007543,
                            463225536391688408,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18274513633322999197,
                            7165038535871059952,
                            5296893277260269221,
                            2421305131719586744,
                            13058838746739291576,
                            1129911328936591389,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1452074148275413176,
                            253517005792379887,
                            9889456999658790348,
                            2139576114717959168,
                            7439043430229261887,
                            1475740478566097248,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15661538434425882917,
                            971677572174623317,
                            5425596419450720164,
                            17937140564850191591,
                            12663984048921288312,
                            1351703496531388632,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7364203494592516438,
                            11468364815586117904,
                            14118104996076872486,
                            6001166368081071924,
                            13695759462986195410,
                            1300434096886380372,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15822649961347786769,
                            3580670598919003671,
                            7693182822216600534,
                            16277225726105488061,
                            11197118583593117420,
                            58015165862658380,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17790198068728695860,
                            7234458935014357,
                            6645308713270276386,
                            1108965528716371484,
                            12383267390965539511,
                            680934450938463541,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7108751515868695173,
                            13681906071608324747,
                            10306804698501353629,
                            12202873431168746666,
                            15133301835759971511,
                            776431533383261389,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11509412196563824554,
                            192340567597275869,
                            5624786804019609955,
                            6977812486287659700,
                            15635469719964212210,
                            166719666163808473,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12077570478074990842,
                            17711703566155220638,
                            939474816353470333,
                            18145514887238235953,
                            14203996091400231011,
                            1359844821074773525,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10311604063728649511,
                            1609651387857124885,
                            407701348737146113,
                            12028285977006890812,
                            11046164876796102767,
                            1797722042950904141,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1427879064635928480,
                            6656285964687876423,
                            13375145963470899141,
                            16710347741010854922,
                            12854807559074243563,
                            1750322764529718003,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            600216654275545583,
                            5414333347850592515,
                            13425270001663106572,
                            10061215446213342502,
                            10898293112961830784,
                            1581383153314888180,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7953911101668649384,
                            14502371485567639771,
                            4467223998604199670,
                            12953513131685627027,
                            9787222643454963879,
                            1689317564815265251,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17845090528666511814,
                            17218857764041981633,
                            434321486801145184,
                            7761449053373540029,
                            8320959589622445090,
                            398786852714584690,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4758378269505062716,
                            16351704614477589866,
                            12542090468608143805,
                            808605318425639446,
                            3503240832217460051,
                            1842073828575200413,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15734078063991052488,
                            14805981347834446115,
                            5358165178364389761,
                            7797436101080855272,
                            13897454754534204624,
                            1696458052295004902,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16649621874459670687,
                            10429050216862110273,
                            7460627473180391868,
                            11265217268035742568,
                            2735712989743781811,
                            882920157912079811,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3075135809246941167,
                            1089505841100397141,
                            10076243511248066921,
                            2394028537904061176,
                            14754095266737483210,
                            297320587567923809,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5363564499110814822,
                            11867016904495211986,
                            13941697407403803546,
                            7664856459874461452,
                            10933922705689004674,
                            338358933332895847,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10809564516168079915,
                            5737027856357256894,
                            12892628667917951165,
                            2213734124659505572,
                            13092779794933989135,
                            1323409123077895720,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11131448777240685080,
                            6925404385650247621,
                            13622446378718023885,
                            16721307281747360292,
                            8474077144991808591,
                            1340983569203099814,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1528672006511707373,
                            358890670182426684,
                            15883787751024872823,
                            7589192448887170302,
                            15339649564474758394,
                            178286491281225612,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18430688005093571951,
                            18162952206196859832,
                            3494563849312283207,
                            18436405466476566618,
                            7705418686106117642,
                            1295907200500658121,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9864917128818935680,
                            15031094304689192001,
                            4383772968751846037,
                            17381792424027520744,
                            16117299811085039800,
                            1220353500302315717,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7855118772709740126,
                            6540028954535862878,
                            15292103460338394486,
                            8060281849357684272,
                            2633944968963670217,
                            476348405468463137,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17765440231526072324,
                            9676826235115956546,
                            2705546691226375623,
                            2752247537313589314,
                            6877825397110780365,
                            1828782654102598044,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6867090617432332364,
                            3356641605717282471,
                            1705574496795018025,
                            12956011776884799676,
                            12701038320376644602,
                            1533226673826508571,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11842521177258981316,
                            17960613677823993081,
                            17452903803859729890,
                            2998747472929348727,
                            15666480210421280333,
                            940463773828130626,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11370869085942358320,
                            3523741466631425292,
                            12392458386958233900,
                            4512654000554617645,
                            383560161132585504,
                            688936409456362894,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3654900830952232285,
                            15816179799092201856,
                            9701466186991521955,
                            1204285215825073102,
                            7196014837569680243,
                            1807440671971120011,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17446673905646733460,
                            2968035069490327605,
                            1945422452553785097,
                            6405861100054680345,
                            2026663606117160557,
                            997594189683734898,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1827682781581716425,
                            1794243432359512807,
                            10605178223650739435,
                            10180306364887260735,
                            10579069175736778113,
                            484225598454989429,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13875111464225835617,
                            7691967656974318333,
                            1852918340622303378,
                            15864034991226998710,
                            4393910526263175436,
                            511665796839647158,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4783884378215718113,
                            4540621619551195186,
                            2850751512615260323,
                            11773064065889588953,
                            13178808825093108581,
                            326694971871283252,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16487667098820688560,
                            7710334756817029526,
                            11101831390421135721,
                            10347240302317568005,
                            13174135076480492483,
                            1323788570128371160,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15197103216391778603,
                            7110760572534454589,
                            5219725019100337327,
                            1512717616051337044,
                            9187078406523764602,
                            1764851362557002371,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12608100880885305675,
                            10953686429999876952,
                            12543112497099631400,
                            2213901796316081859,
                            15460166960038713006,
                            1256809923215280715,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16631344828141006385,
                            6254972786522134245,
                            10178119461507741560,
                            14045843476639986127,
                            7656771492118495314,
                            1458377839318352273,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16063330626818798199,
                            4650541829620065595,
                            6244614876404924905,
                            12395055914689021083,
                            3580369191028499350,
                            189068726599598796,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7912908214663280303,
                            1244539527153588649,
                            5217899939295510267,
                            11246602870937322544,
                            3496787014304545030,
                            22086255942919046,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16660790545462286755,
                            15160923557348826284,
                            9102870716234932206,
                            5629797427135841108,
                            4563311680313564885,
                            1739212864458319654,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9319999008871529158,
                            1124441984610024382,
                            15105338731927575319,
                            10340048231023281141,
                            3861773543980233727,
                            347834781517311721,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4369015729079879062,
                            3940604960755961225,
                            13659005392738124966,
                            14491388806605099967,
                            924775484483583890,
                            85755882404132494,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            374519509947936737,
                            17862280374400650099,
                            9668557401249605416,
                            8272969037950968500,
                            6717722522445555135,
                            1030247574245112396,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4961476162914894405,
                            3854848957015663757,
                            10189214470214276238,
                            14205229339709781387,
                            4417218397329879373,
                            1601596046767932803,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9191756317311960181,
                            9444864100088947659,
                            6855189798868853640,
                            1471448290965065107,
                            16738530168606230153,
                            361184433356738033,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12498591122706736175,
                            10563461173567731152,
                            2925870601692348119,
                            11739092067567402164,
                            3890774621640006655,
                            988568926197161883,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2851434790580815098,
                            14798034404149098704,
                            18334627029335818218,
                            1412439668022549114,
                            3330133603868618930,
                            482481860756062252,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11076480461501329533,
                            16425796211568415510,
                            5884360154509548442,
                            12741764688653511774,
                            5160897053009606772,
                            595985964684473617,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17481501968826625055,
                            6163383781568494616,
                            18140810454921260541,
                            7019729729609510306,
                            747657912668422666,
                            921218748400401184,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7910412861657422431,
                            14299206070802433860,
                            10904934023720001907,
                            13410482876149709820,
                            2384958978626215837,
                            1761350086503203196,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11286239120939214170,
                            10480930836527603110,
                            9760679216982824116,
                            17730897538364837727,
                            13233235315231713501,
                            667767427353596108,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7540281879100466063,
                            6000132550567242042,
                            10322251115203216968,
                            5574113681572046487,
                            9677027565739714424,
                            87224451523626278,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18135865025704023367,
                            16513046342324158514,
                            16215105130421488099,
                            8722284556001790773,
                            4993224110182347144,
                            1374636940574782372,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3396784971714920252,
                            3669075242798289615,
                            13180640745898139643,
                            14578171727536928280,
                            18223913146611729625,
                            1436390993491948246,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1131222950417859892,
                            18437492094497893295,
                            10228698798272953367,
                            15266850504068439050,
                            7440002742821659916,
                            897472557427209111,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5703884490550046660,
                            4276425947477962706,
                            5888204514887075484,
                            190362201139587780,
                            8825201290298479522,
                            1589658797046145042,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1693681404120296262,
                            8107007286509222730,
                            13880247749171669562,
                            1552200204120423577,
                            18307963471833595549,
                            800387677220048762,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8945314065479858363,
                            17536932884072547343,
                            17606452840230861237,
                            12445471600583414576,
                            3153945872818689304,
                            265371597851563234,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11807825517325396323,
                            10434780438827269225,
                            14921102236137645150,
                            16397929080195504057,
                            8941793830034260256,
                            1240671243615941116,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13149842342568918047,
                            8419575181823323159,
                            17926972879034340116,
                            16683577246991711257,
                            13248515901044517011,
                            804124383940895913,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6828471912622968418,
                            16852502461087490797,
                            4917095303035664428,
                            16584919885452493822,
                            13526296253423517608,
                            37068943455264989,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16429150428479559360,
                            6090358362583706423,
                            4337862606197229482,
                            17316819767602712848,
                            13851062445815223654,
                            1552858673892465146,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10476947128839422004,
                            6419637758485170305,
                            16788849621669907831,
                            7018744208196360573,
                            15149867409125287793,
                            1146745591388616820,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4218854713723296212,
                            5746676787536371490,
                            14664483290659652970,
                            3132817558113706909,
                            16066778071633692760,
                            1547179964279102902,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9604492757616667993,
                            2115876842065284597,
                            8700275535062844165,
                            15456997435423204979,
                            17942869472006406032,
                            1868140914031865148,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9601925843077376948,
                            12550574767746444407,
                            6308390619541534427,
                            12831025143367490374,
                            12739863380576549251,
                            1246186122007818689,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13357099077526248157,
                            10927982722792823798,
                            10355180012280029484,
                            15157294740713617558,
                            381448918254113853,
                            1100388803663847999,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8897621459645018997,
                            6607841315862345,
                            9313793248558125897,
                            15624010497408979186,
                            8671122233306691565,
                            1001600543736992533,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            233763342586661333,
                            18179496807331625382,
                            3454653555867713179,
                            9531316003525379528,
                            9805407905792566628,
                            619082838981481021,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7466111381324945590,
                            6613179136665616618,
                            16537610072355960083,
                            15321234996977351684,
                            4591215788778961810,
                            180912637343132148,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14908397204954739241,
                            17752955875838930311,
                            6508116482146656851,
                            10162180830810913835,
                            1238536751827215150,
                            442432529196498911,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8685430211725868320,
                            14366536341838877914,
                            13667556712476441866,
                            9593963582625335931,
                            9588752782420041864,
                            1510953500679319808,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5860083087492309854,
                            5354396228463740899,
                            1425301045007190601,
                            11427985354370956058,
                            17281790371390405961,
                            1738975714387803877,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13971116820166620450,
                            4037790649053401381,
                            14000789417085376619,
                            11786449676905252380,
                            11205693916923363934,
                            1145122980978999318,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2362525035475607372,
                            10790790573449293274,
                            14709620761576913869,
                            15561886936187009288,
                            17552799920143622317,
                            906524521413690162,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9148580777855278525,
                            9201619466325120746,
                            12222303873948797905,
                            15994685648620839506,
                            9012843103690271364,
                            349590262130508903,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12030596403131774691,
                            5400724023463431545,
                            2007520327483956508,
                            16501441191786797857,
                            10580150472454046540,
                            317051644950151745,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15738735580091264303,
                            10740382260027257510,
                            10061537908143741675,
                            11692763105448096918,
                            13832024148065089819,
                            797755810753465466,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11841580167713483080,
                            17433056336083848361,
                            17600141014603514811,
                            7565089146036459146,
                            1048358437156680825,
                            1451719748835728003,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9512987442857176271,
                            4602400208090369990,
                            14355585725269563137,
                            1458942752130196069,
                            8648734163309770138,
                            381272633757044507,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12230123046173387545,
                            10458544878303856304,
                            5008678336870036485,
                            15145867043167423229,
                            9722102766567012074,
                            846665048281601968,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5555457897525984402,
                            8970910994860824309,
                            13653925456085361596,
                            5701791226860105175,
                            4954555471822208440,
                            346474221626074939,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11268586486507521827,
                            6249786791175931661,
                            569118417960980033,
                            8473695511862241269,
                            2178494136187404976,
                            431432387993630182,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9767818281574084775,
                            1391450138935987945,
                            5174593879919979238,
                            17756379008181005275,
                            13001876029787805667,
                            1489785183613606589,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15585933096465434235,
                            2121643990888102447,
                            16273683044007784467,
                            10943999900411221160,
                            16688262079688955122,
                            1595420476499348304,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14530381411507886250,
                            2914368546624793584,
                            3110585286439612066,
                            3117909033431974203,
                            11628027506324341403,
                            1640143277118629030,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11061033095686422711,
                            14510911052289775026,
                            12034051224375857437,
                            7428201691196570824,
                            6105260842572334121,
                            1451798730407105563,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16348332367934394563,
                            2846386206927834309,
                            12710883081606547466,
                            10238716325422620291,
                            13479719958085072741,
                            1169039471489542700,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15724428694805818516,
                            1118188822170890733,
                            10020289466996559698,
                            13196527384266001734,
                            14229921070597705286,
                            453131383027160541,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1886492232467677427,
                            9155808694056011525,
                            3855631134365137616,
                            9746157117637807777,
                            14598754169850167743,
                            1256752014320216960,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16900211377735904598,
                            9541207940726013150,
                            16142430519856260861,
                            17448239548293786731,
                            12218177147651901902,
                            836840490789639547,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2886495440003846356,
                            14727056408252187117,
                            10767092398945987636,
                            5949399035132223748,
                            11110114335930746748,
                            266015121701179733,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8480143601674336423,
                            15912081855712111904,
                            10935439334081792176,
                            10240870886296076087,
                            7783194744692064149,
                            1131521111249516224,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2558463592596876794,
                            583958213346151011,
                            18014542345104299437,
                            14956299395006892573,
                            14864400237068714598,
                            1595584330988464848,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17446564942590282068,
                            15429666926326422399,
                            15694117050543498354,
                            17826505725541075824,
                            9405564886934799689,
                            443256447757001408,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18837462123398949,
                            6536924409262152549,
                            2675833911653007748,
                            3410516367434162110,
                            12625593918426696197,
                            1541303055058449767,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13750684621528149303,
                            12629231418344635697,
                            1857508212202154218,
                            1790093184673799050,
                            15121411427550266970,
                            275438851450726136,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12276493280963925407,
                            3270358995025284257,
                            12091953896214483072,
                            17910750599704837367,
                            14264475547154288821,
                            313143896161381714,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1332146746383296118,
                            17255736098587014279,
                            14816887651067907559,
                            6055520764886473228,
                            15317148872479131991,
                            871437953543889679,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18295607816066475325,
                            1067084375565372712,
                            13673520917124049324,
                            10695501982698404256,
                            12996018413156863904,
                            1325536126148830591,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13924501321009869428,
                            8634265851493182010,
                            4511029830559118103,
                            3568064270776750677,
                            7329112494530347235,
                            1532056137037072987,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1427023320611130676,
                            11960349193676796020,
                            12013030065653224620,
                            11872273267124159146,
                            5995878413815735116,
                            125875327340602563,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9049564332870174035,
                            10378124515149494580,
                            3324550182692375582,
                            2885173350661320828,
                            34625098653913189,
                            50536490812094212,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2139290107079644491,
                            3285366231758916967,
                            9776713007062133069,
                            15325239047839657685,
                            8074746099920232094,
                            769784250016793204,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7013078215331620411,
                            13516480407259643604,
                            8172543037750184157,
                            7102370402364549573,
                            17550140175781155590,
                            1820141719382774601,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16977843566776350591,
                            16597733624496371182,
                            482801885502878834,
                            422230165138351313,
                            12194242086364288008,
                            84168290893678564,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17746708234002147313,
                            13890082798757834104,
                            10486693427777579699,
                            16630505944025832898,
                            11209831055963795396,
                            285998332440715959,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17942077489794309785,
                            17965711880365933462,
                            2709080271199751297,
                            13524235603042559027,
                            11928075245847959425,
                            1582647677606540166,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5061566519242798271,
                            450345864977296986,
                            1926462603861529059,
                            3116689399115277034,
                            6411159212323990026,
                            455959799970799358,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10444940123659646236,
                            15112464026327313603,
                            2920813983989772818,
                            310434131318387232,
                            12881944387060054284,
                            1236393594208692022,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14901157661291898361,
                            940142789539996102,
                            12360162332234946476,
                            1504475837926657830,
                            2997137965923495864,
                            1263219474182695744,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            504969174943307686,
                            11128967441430027986,
                            1162156125259854108,
                            7755981445530474051,
                            2039238058185356728,
                            22586689550108379,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17961087863949889222,
                            6422430546967149933,
                            837008314542894338,
                            11276355777110081806,
                            3412551651541014448,
                            841337816078151637,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14872825222809788877,
                            2393451172414746646,
                            413538491764256539,
                            12933591748032011568,
                            3351581556648283258,
                            782596482577880205,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            504746258239733438,
                            12085326855307455916,
                            2280723197161719455,
                            18422591298722725984,
                            542390479264666327,
                            440058484034435178,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2708755935376602004,
                            9132803234669402130,
                            1977009408258888207,
                            2358367280359692710,
                            11294311373456056947,
                            1029617061087637256,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9481172343655531671,
                            13986220110339277816,
                            12677059319686112944,
                            15805358118076412931,
                            2487348979304608583,
                            180250936137238246,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17450045029473327362,
                            11526951174892935519,
                            15388435877947052910,
                            8472747805280844831,
                            651440017552360636,
                            1032884450055211391,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7365176830944236124,
                            1407264094595173047,
                            1707930433331235926,
                            7095441601662635015,
                            12966215563198344774,
                            216601671243634782,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5818602704773162449,
                            3263430439549258426,
                            12261106858523380353,
                            4532096764148239217,
                            10588692139859299584,
                            678860834220986584,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15886013249971217740,
                            15749120917128221094,
                            11391223627947230156,
                            9739960937417431216,
                            11930629035788909358,
                            1386042796129958645,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18009758323768426626,
                            15688526785026375885,
                            9052672553787857201,
                            17098084525049714177,
                            3429682098614426773,
                            1635586046779065050,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14480615821344381108,
                            5320916422073334579,
                            3874323348917314111,
                            14239419032624646267,
                            12170702702485978593,
                            62667813100750931,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16633532025534818497,
                            5105605526354159200,
                            116371485141499325,
                            3563043692527578234,
                            9743280782433467986,
                            191734893958064739,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17787579823268465968,
                            983147809418282055,
                            10436753112367891997,
                            12051421020351637881,
                            12808283427148587540,
                            946523263266965056,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7557753804677190402,
                            1868689416192226865,
                            2866793430929842363,
                            10579741929816299011,
                            4224179535915609218,
                            522118784290938149,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16049443653289505579,
                            6234045106681390982,
                            17112958157902276426,
                            15933239406921575515,
                            763747015067199370,
                            278448898884693470,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            597828249674798581,
                            15524987575546089282,
                            5821577246680231494,
                            16101698029334489436,
                            16425445765084053140,
                            154490486092510850,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7641175813644237396,
                            9242639387815443767,
                            16868651575364005258,
                            7632657546518119079,
                            17360613418510436855,
                            616010438738443515,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8908622966350013699,
                            5172610524429074474,
                            15319313558257893918,
                            14141228860593512325,
                            9125973363051074026,
                            1025929757195792118,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11079793026469425878,
                            5414435832040434577,
                            14077833142206707810,
                            8777731868979745596,
                            10500096573841787115,
                            1221427165298938941,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4870370613682753556,
                            164177066710720218,
                            4885021811586536205,
                            4281156270537918073,
                            2510281751820711505,
                            1595488243230804195,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7483942331952467359,
                            2818612421376491431,
                            9635005131040379430,
                            9236791352905058808,
                            7286334090988254649,
                            1847335700136917976,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15789678139086319610,
                            142097753968991594,
                            16388870877495489255,
                            9569490384952355680,
                            1031136515789373161,
                            1713253222209724456,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5785788737519094651,
                            3716212816220847352,
                            8604842845722864056,
                            16599114859176771243,
                            1145683043225518700,
                            60028473766776274,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3250238917270767807,
                            11464160087753305710,
                            599576521734199650,
                            6557388830862983125,
                            17062214447675692798,
                            877391496045873843,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3755319647652493377,
                            16083655437212405702,
                            2193672529072280075,
                            5996594255592331426,
                            9429139041287621803,
                            130205385792630692,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10383081724930109072,
                            12593861536066275986,
                            9092363334153095535,
                            15574659385312906131,
                            17215748711877627119,
                            1040145223288196417,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            643656109511062601,
                            13200658324373371178,
                            17206960108259696188,
                            11233738938246382685,
                            1055383647862133087,
                            1486396485995704042,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7634501443760909685,
                            834077210104955412,
                            9929619938206958029,
                            17890760045475262385,
                            2899753784719029241,
                            643667204327897670,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3174268054371490465,
                            10009757553528320724,
                            9159224366562372825,
                            4159108474050767996,
                            12382873007048194202,
                            86929289079880902,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            934716554789158677,
                            11794576519459193490,
                            1977297271684512703,
                            14651667789677145666,
                            9221423271519112098,
                            1017589536715774935,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8056025269452223405,
                            4079142411217425591,
                            15019148581930066391,
                            5618626673969278649,
                            1350278659055051921,
                            801622961883930833,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8846977164480037261,
                            4622977331006727655,
                            6185667009816791126,
                            4677603835163498196,
                            5955950476880706146,
                            1326492631786243205,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2967091482354529926,
                            11129988023377312234,
                            12676243656597383665,
                            1817413047939933819,
                            5644784580405798802,
                            556036227602466225,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            85845160107860753,
                            11247660396890151242,
                            4345592639492677279,
                            6734602623216239891,
                            501496018454030390,
                            1305232414753438920,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11148234907557086529,
                            14462342730806719817,
                            1785717931334717881,
                            7113502317257651687,
                            8809028124916226331,
                            180149914168708407,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17211462816625525513,
                            10067535613947661868,
                            10048693001446878550,
                            6426636549308521581,
                            7886927805009381956,
                            1210983796369315321,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16131450706276282561,
                            17975046937463708603,
                            4939893144681836925,
                            2534853622172135159,
                            18134417849241909481,
                            1039396520715138618,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15026916231730850029,
                            9811555623371877965,
                            10967470596638318856,
                            15048633004968624682,
                            12933825869646574721,
                            659583207878584315,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15244656217298296931,
                            9430831216212058572,
                            13952810780591350141,
                            5119385460660822351,
                            5772706368926564439,
                            599415181697207289,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2002581819720380730,
                            14785763027960546945,
                            4005577012622447306,
                            12289127541534828740,
                            14349838012616278704,
                            70983718740382502,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            476033296859622683,
                            13384601279375565993,
                            13068138190975066339,
                            13127375079578290872,
                            10764493554294759566,
                            1381054862658855899,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14401491584607695384,
                            16611457376406063811,
                            8533329089852311094,
                            13714113107816451663,
                            9385048128150139416,
                            265755515749398785,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1758081742691861576,
                            10029471554926535266,
                            5579104807043479358,
                            7426552216895414407,
                            10603561323263022992,
                            1828359084145060288,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            671201617396467918,
                            11270714995297395167,
                            17595200338417156857,
                            2997793723051414075,
                            5476069598428923542,
                            853505663081356732,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9475917557857477054,
                            6836565551475246463,
                            9636318994271348965,
                            9083220575923864558,
                            4655688152754256170,
                            1157663155294124729,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16726112399653613676,
                            17608677981924820831,
                            9403451788932647702,
                            17097169495393523745,
                            13916955309651939508,
                            229929295176615425,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1500654658363091166,
                            2462418948768557014,
                            2522355064324754832,
                            16948450337446527841,
                            9627814987936906615,
                            640898026263877898,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            247391394247950715,
                            18141432130890377810,
                            6517003376191310645,
                            17762250562540061071,
                            108722657843843712,
                            1445540051939526682,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5157899701727204085,
                            819875820322144099,
                            6268223895361764007,
                            17790643446513710876,
                            4856090573206057831,
                            120037224608867349,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6673596548810686211,
                            15652467327699337602,
                            16074380168166206423,
                            943423852215763048,
                            2206713156429161863,
                            1799845403716403460,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13708345154112971142,
                            16599240941408280833,
                            7472377757881781340,
                            7041185008155302587,
                            1149387447893298464,
                            199231073305616386,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10638864930094125391,
                            2964393293098342998,
                            16051437604211300379,
                            1823878552870796014,
                            12693670859951961957,
                            1323093107826232289,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12502883135788010058,
                            18231280322420514288,
                            13609500745263941821,
                            11262232747352697250,
                            2050276133421719704,
                            1773868523542181495,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6582546930739979992,
                            18064843125404450922,
                            16879886621970926628,
                            8304208433710264221,
                            8793142427366929350,
                            534269152404257089,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5381709330803505803,
                            13511745777493235733,
                            12518695074374605081,
                            3906594895289128788,
                            11980367190388171894,
                            87064433719931293,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5818419501413550715,
                            15350176950142879874,
                            7564850867650072943,
                            3837768924741808582,
                            16236069890090141234,
                            394603172328733518,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7281154305111416769,
                            4487193904832681241,
                            10482057769478292600,
                            14428836532944178231,
                            3818847610768323225,
                            1227705540953012991,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14469093430695529441,
                            14177409397091967112,
                            10737765202579359475,
                            5700512764417476765,
                            13705279009160092147,
                            48010097963797291,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18431134707601693210,
                            7943687374407101956,
                            15518754531551990431,
                            15393129583099572613,
                            1737533709702992939,
                            1064475304046392789,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13729905244641979546,
                            5257781694625282910,
                            6029432367105817497,
                            12303288450971948133,
                            15204568792935025486,
                            45481162493934815,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1079535808577504745,
                            3742438347222741760,
                            10687136529202576920,
                            10540734518387687769,
                            13332329698881989601,
                            1627108616201631915,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10072627604046774772,
                            11207552325089573686,
                            586741396402630293,
                            14934936009771664872,
                            15310008035487557227,
                            474720856878311591,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6678308864680023051,
                            7763453032544463201,
                            8128017577434783976,
                            17543744881486595866,
                            13844081870689233043,
                            687122574369410419,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15708340494683514223,
                            12914495024836956765,
                            6268353057240977462,
                            18431470635286417424,
                            13809535140037846890,
                            862571744357689151,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13511797885789807453,
                            7518003312699334915,
                            3162338503116824862,
                            6241710452729169149,
                            7094359365544231716,
                            1708378573776828963,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17449622493173814478,
                            15762920789993705289,
                            7026913244474081236,
                            349265112885782541,
                            4262810789085200878,
                            967636617801196363,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16070710155619574344,
                            2312512311948067641,
                            9500122922609539086,
                            14766935065840947946,
                            2865598743273119825,
                            1141847869534817919,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6344135760330774299,
                            20237546976340208,
                            7967698563967042030,
                            3564284165046232581,
                            11541660314872288090,
                            629427937028409768,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8345858857333226225,
                            4056862673494730895,
                            17652874393747873290,
                            1371276796011047660,
                            12325247121421736885,
                            1697494236265527758,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16058133463755938053,
                            15162109911118906467,
                            16875194522327383112,
                            10810057582694061779,
                            11120818845384684813,
                            924321402668763083,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1854362641616322716,
                            8408015594449886194,
                            3230316741017159264,
                            11824743867488752532,
                            8150634807842722352,
                            876312913867229724,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            197223997876536312,
                            16785199390686343123,
                            12260844578667807714,
                            13803799107428906903,
                            1555835140070862158,
                            767588343461276013,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14114696892650114167,
                            3226855278551472181,
                            11024518779862788351,
                            6440252271396310950,
                            13484813552917308041,
                            1254150784261742111,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6915198207777947371,
                            15825608317379830921,
                            10401290094720000760,
                            6874370291552641933,
                            1214404880067585053,
                            1742676671247371751,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            554681470704440006,
                            6305619944227277233,
                            3386900348683362144,
                            1949354595813508691,
                            6033074702430730414,
                            1698422090901230114,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15338222914148316010,
                            12377379407403092210,
                            18136960348538793649,
                            10043720685740566309,
                            1734708583794832641,
                            1146442558765030126,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3933736645647854909,
                            1944746901392429781,
                            1751059834539120114,
                            16421765145861123633,
                            2265600422991001998,
                            494344181283028521,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5118962626562096639,
                            2144270977124034316,
                            12024023598645748383,
                            16998128793992990438,
                            10999046712007634701,
                            1361754309230103645,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6267223636222920017,
                            8526101310420643506,
                            15459576729558696335,
                            436566652045809141,
                            2429938127522542926,
                            435970476903457622,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13685266480877902990,
                            15039697852170739250,
                            5999162652258661846,
                            6969890845429593175,
                            18017883970456866087,
                            1128093971018768292,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5616649208306484642,
                            13767263204138992060,
                            16484465342725903299,
                            12219549762887523625,
                            1625396102194408524,
                            270121101945181972,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18289040732904181887,
                            15787149131149471230,
                            14412806396884708696,
                            8449362377510943851,
                            1870032235507410041,
                            204814764962193692,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            280133190698556498,
                            3778818680279306934,
                            2045929580536952593,
                            4235331680543425849,
                            825164297310450770,
                            1847679367551055015,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12887714432874279068,
                            1993020441992937319,
                            16615123206253789332,
                            7994904475784341479,
                            5550825689012316165,
                            1379114606266609437,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16121913258908632883,
                            4614505332686681323,
                            17127857512475675520,
                            10135365154313638546,
                            17301246298875399676,
                            1245756524714845376,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13138865495269074727,
                            646099245207820161,
                            9912924189901660391,
                            12110822414326166570,
                            15515634970411191436,
                            1576043940298855074,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1869087875635714226,
                            12625678309312697025,
                            12615537621252666780,
                            4739326373587847475,
                            6123844338746918679,
                            1045711955779092085,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            882457720514772761,
                            10973943279125443435,
                            18063061459754070027,
                            10180593414612779714,
                            9830274087881604259,
                            583993293232020218,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5368352291482319937,
                            4996336871969496195,
                            2953096495367400804,
                            12329385541545832713,
                            14002200891566843591,
                            1389216765602956032,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14946012133818206715,
                            10141259852011384551,
                            33278170267745070,
                            3984795230858631798,
                            4027569870748205112,
                            587045876864920097,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17495715475323225013,
                            7670640758535971419,
                            7558014606165167629,
                            6139318875120393903,
                            6312105094319666476,
                            1776297276129350489,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8134551852788465503,
                            1689607357525963663,
                            8689043304186027430,
                            8969311778930272337,
                            12540459095085462107,
                            1014605753358060154,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4502095076386086548,
                            5050834343927355777,
                            4016625433877877139,
                            5563823549816048622,
                            6057653528621705281,
                            1867082322999344654,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5233686668589511042,
                            17141650195475450845,
                            166992622877609082,
                            15381605692146435359,
                            10095418594825252115,
                            1872305201853021815,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5715186471318322136,
                            8698492431710181663,
                            12370452617135377150,
                            2719476964473939638,
                            11981857684015211437,
                            617419608386868309,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1181634076498650045,
                            12893660024444163034,
                            3099190990095674577,
                            12979830602341511159,
                            14062503947707491385,
                            1011999948583898006,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2450070326687781037,
                            12982070751772574408,
                            1040643936794894602,
                            13146910064273945743,
                            9731588068563082863,
                            442570562915431899,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7425340590144792587,
                            172125985796190288,
                            15302516775559034981,
                            8374743388915249370,
                            10816207337592423964,
                            448611880336218988,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17309354681187641219,
                            17267001949163141710,
                            8640344944865267622,
                            17587614646529357222,
                            10574819041696652071,
                            787653493874810889,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10811892912936886699,
                            15356637348539678856,
                            5000545707805493727,
                            408553627986629669,
                            13648249379410911295,
                            869202766936012881,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4917153853665937694,
                            4591895061358218733,
                            2568874911293566849,
                            12364868639553041450,
                            16680842968024819102,
                            1749589204326980387,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2532926491184531262,
                            7660483422673682211,
                            16061217831913460207,
                            2068491925341409417,
                            13051244506870289604,
                            496576386106130939,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3472931335207165603,
                            16122098576363886020,
                            11909443469582056783,
                            14896728153095846109,
                            599116321172746805,
                            1091781388399734948,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            810552905219561937,
                            10610909402484210023,
                            3175059121281288667,
                            784191003879144897,
                            5320031841822975611,
                            1128532660309980813,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8090659203129451420,
                            5976755188618955052,
                            15883870818637195540,
                            14723522156689350004,
                            15897754911187980226,
                            465399857996247438,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1544337406725197852,
                            14659322697913457535,
                            9499355068095146482,
                            12838139958102278049,
                            10146207690633130990,
                            1586750250440934206,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4644888960680794565,
                            17435237999669924691,
                            13521018777083760103,
                            5761662352700362266,
                            14366743612928538283,
                            1214417005904731601,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9083676387730382050,
                            2862079173246843741,
                            11146235748606059077,
                            1963755046967401694,
                            12470009976570297794,
                            739726183813915258,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10326154043874153551,
                            11080934282307286683,
                            16614876120027577392,
                            11704097468561994654,
                            10710658641109872492,
                            176955610690441238,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12950157156629591466,
                            8660480216421198185,
                            2389621936687223798,
                            17369961952849224403,
                            17182729343216810128,
                            723302451967307399,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14654703356625934821,
                            10191519416613522847,
                            13248670481249646168,
                            12036411318356251275,
                            17036000999030271523,
                            1485671754685594446,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14558430125654880570,
                            16692834336527054842,
                            563619496781494127,
                            4827752127228013615,
                            1970133768231523606,
                            1181032790779799866,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13987404521987220051,
                            14023631837876838276,
                            14103300333705511159,
                            11986241301751934871,
                            14228400315885434953,
                            946358201980947736,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3399596786106383105,
                            1924944913898959053,
                            12173427276603060706,
                            5631059286124338214,
                            8425369771105643759,
                            996022080297139170,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1104222252884166189,
                            12655961089316906303,
                            16120603810325708425,
                            7576028799062140842,
                            8952396546508063030,
                            1304147764857767913,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8878921254315022052,
                            10132593357834713278,
                            5537418493258692541,
                            2410537735181312771,
                            1949335088203829731,
                            1203694364701300098,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5239905263861076123,
                            3262109898944242083,
                            14839072735826911675,
                            8466121870778428727,
                            11853974005987303107,
                            1063557961927081436,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16377504179782226581,
                            7506134599857010314,
                            10527855756935590536,
                            1378021226598719761,
                            16486038852734948486,
                            1555686612739665487,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5679708117390258412,
                            5688874111927024452,
                            6229501604523844561,
                            1750032931398730793,
                            14181329066398998493,
                            379977079285420356,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15662399217801968609,
                            6048035434448639239,
                            5630179038397128660,
                            11117163336615802509,
                            12302032663427302883,
                            1477946072907754827,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15389850345759258636,
                            16291578245368095955,
                            4294249060545283616,
                            8597665521542404519,
                            7313901830431988176,
                            1215682245601995143,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6783147800757807260,
                            15162148364859997143,
                            16804990876003077337,
                            5441803344932848224,
                            16345171841109060813,
                            202939260088691293,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5116237634567322794,
                            10872153309813384093,
                            9953534471995415091,
                            13284095475912227440,
                            2332382355572116571,
                            1053802884726596155,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17337143158450453261,
                            3199543596041476861,
                            14553693077906423482,
                            13806859823723296226,
                            11221934611867095322,
                            1136177245128119247,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10508604019832928464,
                            6462950433505833930,
                            11605023749576199104,
                            4667511931768510515,
                            11442959725857316700,
                            1245612020733295128,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16687496692825708414,
                            12947099845203553678,
                            4261864738020385556,
                            14757320465704151934,
                            13991391596297129432,
                            952323342034675720,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17619974543261149967,
                            4647545239967497899,
                            3049501863101711115,
                            16190933827568666077,
                            11377457396538403945,
                            477025103779147991,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2174601967961813440,
                            14003100917396634791,
                            10656961743598294368,
                            17896406508490072061,
                            9587941194287696360,
                            517632557552719026,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17821979251453980652,
                            18028442356806015602,
                            7747368644056824628,
                            9794935605082714011,
                            2339436192992715622,
                            1117816833841480708,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4033025756444186313,
                            18319079473076136551,
                            16392474719170793146,
                            17114807426477747565,
                            18041319656189892305,
                            252677953690973544,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1553304359603834276,
                            13265792566329450641,
                            16784701175056530299,
                            16488177676744129527,
                            8125213623975158405,
                            62007143183162309,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10289955547454482020,
                            9640838143798789489,
                            2999980315192642898,
                            7466946737569225331,
                            14839252287731116519,
                            1865180699185853661,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1706182033915909324,
                            4526674676922525067,
                            2839506378847713791,
                            1456928182645658316,
                            964257771392168070,
                            755546041724734435,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10894401738371107919,
                            3389903128073247541,
                            3725080878592153627,
                            5605618961428656938,
                            10787725952537216109,
                            1204778814229792763,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13306850128689404419,
                            14038949299257470399,
                            1989297041729415843,
                            3907666303327386774,
                            5702132837718461210,
                            1814614746844359257,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9706157989658202109,
                            6707904054533140422,
                            13620250788403079950,
                            3465688604745861615,
                            14961248428341176214,
                            1388946301442846477,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17799992878067095741,
                            573376315489957785,
                            1091970360817600977,
                            13968644181338953999,
                            7107568145266615816,
                            646632040378645023,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11169800552109351904,
                            7902171312406441467,
                            15372191805514302398,
                            4832506482253619462,
                            4931274726583789383,
                            93165863272683935,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14886573267805118598,
                            16638086051003208400,
                            12436853001115248191,
                            17568688696004970844,
                            17332495567050433522,
                            919970811992722053,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12716895416657346462,
                            10202131567267535830,
                            15724148786081578380,
                            399008555079259475,
                            426868679925159448,
                            1084371237692392493,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11058696917143630454,
                            3545171992888911575,
                            17135493043601304990,
                            17989230716983636447,
                            13840317626270824332,
                            263656297276193444,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9541473205655028866,
                            9896531692254145687,
                            1411096701430892324,
                            9762187333682248095,
                            1301790740859366654,
                            222370890005455295,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3102569695375517393,
                            2052738540110078042,
                            10444243642207034307,
                            6717290724466642155,
                            6227630021686407047,
                            538400932182638181,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4191270085385413910,
                            15227991169724101538,
                            445319281759940909,
                            3082807275698717015,
                            2717095455568957383,
                            512739157931507462,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5770967957937789875,
                            3961254766652045551,
                            16673602170065318409,
                            3607174403950524724,
                            2944473481537916196,
                            1067513558373527747,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5740032107131620152,
                            12893632206393966657,
                            6555070753610957236,
                            17959204762465581230,
                            6943612988404811823,
                            595560117979171235,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14299570474059847609,
                            7028323523143072122,
                            4668712100262978993,
                            1450111767940784733,
                            12979538070303962697,
                            1365570336812229287,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            831206279506977557,
                            13283680363190128402,
                            10377698720536998134,
                            10895635525449840219,
                            8309302110022056362,
                            1667845892873955178,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13975843247890773069,
                            18420282516668455299,
                            17399087878175449128,
                            9456184238449123000,
                            1497237983971240185,
                            1507155913531008364,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7443037524311178675,
                            3417331767331389580,
                            16927302365871702503,
                            6691250376723194521,
                            17741303966924258772,
                            1480426277012005327,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16770322242110146555,
                            11632430720779233335,
                            11038627769468000493,
                            2788169992228190127,
                            2226323320356754405,
                            722080852325323407,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1298998767233959620,
                            5290514810660762207,
                            4482775780116289434,
                            17228412927969163652,
                            5343077702405620895,
                            1473456344756120122,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1720835570524134814,
                            1745291019167735645,
                            4812783616205872702,
                            5524010722576429939,
                            10877960432186337810,
                            893779911068691026,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5866052511707159926,
                            13143728962510655465,
                            11472892078554024966,
                            13373311148430626284,
                            13363229430213086210,
                            647348237169516269,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4587724446897359466,
                            11380925469068653913,
                            7004170327038285373,
                            11782458483943300474,
                            1440862155898144256,
                            732648785928873071,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6171139749188624017,
                            15933502218596984660,
                            17692806511867579730,
                            1986753439343846307,
                            6377649773624493666,
                            864606161201179890,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11571788620836429328,
                            5633333826043179160,
                            10197772716288306808,
                            3875963196393589918,
                            12906385054848034456,
                            1432688216628257875,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12018641939311707406,
                            11764514397104875855,
                            14113939205071251815,
                            7629166027912223027,
                            469301746869792709,
                            1140631647919709988,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9213697485405249216,
                            8497135089351256435,
                            4401742128694891510,
                            247724360260801060,
                            6035234067123914157,
                            1083145856442677458,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18343475410444084725,
                            3408189879444328569,
                            13904044356360549491,
                            2035653532663781390,
                            2092316513159246651,
                            1720732503034824771,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            740750857330866674,
                            17290951286410561239,
                            7071045218522342113,
                            15226697893952202100,
                            7135034157406061180,
                            733043290060352831,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10090623957154040529,
                            5918519551393387902,
                            17497579547334651404,
                            12258249521410399805,
                            4542857732441116426,
                            844924392058993874,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13632662897960906186,
                            1601000296744622829,
                            4759799678484325711,
                            13763264553513831268,
                            16755187756631113771,
                            869034956005664530,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7323699265585338973,
                            3149312032235540148,
                            1747832248554333563,
                            3797107510894665585,
                            5705206586587868786,
                            750752865121091920,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15308728479039927346,
                            16490612234816450045,
                            15469686885053974309,
                            11805176566995366503,
                            1733492213710700629,
                            155194530600073965,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10732017327434726472,
                            18043780640350860686,
                            9765760222555480125,
                            11687047891105431744,
                            11161465730115202014,
                            1100651396542961404,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6337198146553155383,
                            9823025301002940388,
                            13537838126345124885,
                            285915648223734870,
                            6345676952136221137,
                            1709362093775295491,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16013725053069331564,
                            5229444761181339937,
                            10618127893806684797,
                            16174041854815994449,
                            18420828702542974073,
                            1310581757728597791,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1964699428304435358,
                            8939189509815834426,
                            3687778101678552475,
                            9051869195720646465,
                            433558938563592886,
                            721574528727003937,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8530926130700362544,
                            3828735059469979179,
                            12696364009149009752,
                            17795807224115963188,
                            1916560671183174501,
                            1052422011532865093,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14600147041566190559,
                            1727801246691269525,
                            6418444356305644183,
                            9353333811852003335,
                            6602066764657309853,
                            1317070028800716386,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12668660771442238403,
                            14454685551299431673,
                            12887173444079648569,
                            6803638423983720031,
                            621181536059627664,
                            123720793055541207,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2245004301357318803,
                            628532038599776835,
                            1037607080275848321,
                            4866779744497884033,
                            4047526254897212064,
                            1283131087529221391,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7794117941903613125,
                            2856640422536438237,
                            14848709550635340903,
                            9061711814102960782,
                            4708255016213260906,
                            216771163196879064,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13582964151230546745,
                            7189978924007035577,
                            5626326822630395237,
                            18280041587218873047,
                            305116474537157391,
                            1235541495814575566,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            360080346312154703,
                            10400880199350719394,
                            8891861254432451335,
                            2411209052337030312,
                            5871358203719919078,
                            1579757400811091711,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13772336095976337101,
                            2127501152214695497,
                            7675711191832635368,
                            8300379861736336063,
                            4310323794911974821,
                            35417139539433487,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4149641287768158033,
                            1644311270109681311,
                            7814007047413733613,
                            7055185217804921787,
                            10365746197993177764,
                            720211091670546894,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16011644299296884335,
                            17927357162931537064,
                            8551178068722706985,
                            6217737474676164861,
                            18249438757375621620,
                            1717870663231326547,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11600407766240395990,
                            10313992171230867853,
                            15267440847320484124,
                            8204580716784240906,
                            10335635778078763849,
                            1520847446055928994,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7195545655994186810,
                            6842147377752942677,
                            3536552057407761029,
                            16391033963145722585,
                            12802868576512704913,
                            1706025662522519427,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13347352306021642258,
                            10091062368568946266,
                            7136649578245068705,
                            15533726729381838623,
                            12005660840000554360,
                            1093529527686128005,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            390934607356732771,
                            10570200995386327898,
                            4607521007168062067,
                            11281332755743190684,
                            14183864238774593874,
                            890796023447492796,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12001788621263817051,
                            2675213454189652909,
                            9149661583835306584,
                            12408362141593423369,
                            8168663480116986529,
                            1505809389160516982,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10596501860955157998,
                            16537212101483787849,
                            13432555603114839051,
                            438813360296836813,
                            3709736952462363398,
                            1464824949626930683,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17552736113305781740,
                            15139708009560240832,
                            9824860443697786671,
                            6189543745044045081,
                            14557282622454709317,
                            1704897475937031640,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15194804300979606240,
                            14325578047290219075,
                            3956536252503852026,
                            16274775829583941256,
                            16993503073645129141,
                            216281155696429496,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12885358115006280680,
                            17350687892069611945,
                            11618461086314966492,
                            8970582011305599995,
                            13337183657339934879,
                            410761622130636357,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1236170347900458680,
                            10906772370928760757,
                            7618538440465017050,
                            11620346463806869321,
                            10896759169515394822,
                            1826909264385187955,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17686441083579255066,
                            12804186928589389919,
                            15864506579779477610,
                            17593751718362321389,
                            5529312760196023077,
                            471709952370801233,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13943163830587569377,
                            16785603236956910351,
                            15602815568012899905,
                            7552401409867314720,
                            1709433543086004397,
                            1627927926884534907,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9723531880103370888,
                            12967812698624705362,
                            9828822199972925561,
                            4046209601369077574,
                            2019593694756205364,
                            566437751600534029,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14725386758468925050,
                            3696968045287533666,
                            12520379808188130552,
                            7413368235903588394,
                            17179012390887330135,
                            917222220028200879,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9713035033791998458,
                            5668154360165243021,
                            10371987663735658021,
                            5769205667821534470,
                            8770710935730446003,
                            1467926411031949529,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16487105651632780600,
                            9729613534130450251,
                            5620813301494517028,
                            866227240634634154,
                            2916952374402250584,
                            1149960554498513683,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2366420970982188077,
                            17456076891084890758,
                            13778010790572053783,
                            8098857293575462025,
                            10716287581268090622,
                            389074389206961401,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17770673664316133010,
                            3594695075798046841,
                            10544801156282601238,
                            1775803355312734391,
                            6816934877486510384,
                            1638837635145417736,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14365438208449804129,
                            9855436038928458854,
                            8930004768911952417,
                            6588401514512916719,
                            18361097911461746696,
                            860727157252820021,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1269011045836919445,
                            9146575631699911421,
                            10608632424632590051,
                            1609320120008576913,
                            9085321674175103363,
                            357774541908665949,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7449219352680216943,
                            10383754896013123130,
                            13474564988721560401,
                            2178813769222568669,
                            13351514473718936308,
                            298706140821169170,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4425607281990497706,
                            12656287974383901770,
                            13849145564838880101,
                            16276108436526678179,
                            15319629416542465326,
                            136261654200738648,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18226641774049765760,
                            12346914742892917379,
                            4337620818074129202,
                            5176720764979717407,
                            1213401781652967084,
                            1361732581275641293,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5670784665769644409,
                            15388742199934761100,
                            8866831872991789809,
                            17196747838808197682,
                            8953423499142292712,
                            287408653766993981,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11371502373839199084,
                            7478706532295036925,
                            16501941539854242824,
                            14430851975329061060,
                            8762837357511141239,
                            700487374362670167,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17559118789399313420,
                            2276060953176040206,
                            8660115808144626924,
                            9416709014650533381,
                            12646907116364725990,
                            388251565012048217,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13074170043333871673,
                            7906883866062170353,
                            8813558480605921608,
                            13382947043739495937,
                            1037221952745221687,
                            1550804670085448390,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13015019525385996225,
                            5017260093973921914,
                            5679095462684575129,
                            1912781607085127184,
                            17912004075220891451,
                            755294764888800885,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8206910184382595575,
                            2449899488828457728,
                            11480296278975931077,
                            7083967460372814178,
                            13661536091773423416,
                            243676108830223347,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9124408032208404087,
                            16782047288035517331,
                            7083973178252296443,
                            2848822970103864663,
                            13517577417110915535,
                            1566686020515451066,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12777866874750969696,
                            6468493148848427722,
                            5237708949112526386,
                            6388349065023771768,
                            8393439894031623453,
                            721537119858626113,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2817627161538419071,
                            12631318860517558773,
                            17256707092034217667,
                            16544713831817882994,
                            10651702336891664947,
                            489930105649242450,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16450633171373654955,
                            11256413568056724125,
                            6013308701914064101,
                            13043621660238502920,
                            8278000531773111247,
                            1259016837333293999,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9636521360516157338,
                            14333871646860033548,
                            10746088978681814022,
                            9562232829090690277,
                            18434673607435680215,
                            1083093717453557552,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2753101063760769765,
                            534572029922189270,
                            14502932421676269419,
                            11822013956145212401,
                            6809473362153231906,
                            1119683640393865749,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            494560408213251629,
                            9931680604122969487,
                            8113262704019298644,
                            13897304758379397084,
                            16850180100477898196,
                            551670905511323284,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12092805204437300472,
                            5628123318859892181,
                            2958706208661759415,
                            13042788039249827219,
                            376428661580651445,
                            1673108164326024668,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11909068217808116255,
                            306255019290189317,
                            15988046665314334581,
                            8201711198029451047,
                            16969831353422249298,
                            1551997886933497465,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15600239978343565736,
                            4872547909284541518,
                            12818323168598072174,
                            7396623617427083699,
                            2570587016561729331,
                            554605634509636673,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8288598135674744138,
                            8728940646733991022,
                            11488070318822750914,
                            18225911695000067917,
                            17051242642254324327,
                            516673391970788043,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17625432568878298175,
                            16015515382830824272,
                            5112407783557734854,
                            5227378299617674814,
                            4170359583142399679,
                            478606780058589271,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1407222416826324392,
                            10158059606824348095,
                            7818044397847761824,
                            10517370192783656566,
                            18267373921571219303,
                            314760494423180358,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6035184501044068251,
                            12362139909554619545,
                            16173712496977684912,
                            8565212904749077604,
                            18282603375954080624,
                            53732736653086565,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13899620657622235939,
                            3646325537770532591,
                            7626056264211646629,
                            16527940030224389136,
                            16982778010583068091,
                            1052200409189742530,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3428061368705802179,
                            9172537495071915890,
                            13179176371284966449,
                            700751609655750664,
                            12690493996632458263,
                            52961215894976144,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9509913533673091268,
                            1975236828540636198,
                            16594874537000038965,
                            13576982874929068817,
                            10725185656497286950,
                            1238021259974286449,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12670536278745260827,
                            11199559527741711624,
                            1731347984091322519,
                            4985587212271287400,
                            16934547186916918627,
                            91796749095206282,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14574661712573087220,
                            6097049098691705262,
                            17877141646844674468,
                            15507493212424683885,
                            10657666408361467983,
                            1307076149042914396,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2388325403923589929,
                            12380567283038434604,
                            604905932484493791,
                            2658975434838153423,
                            17192494081819527871,
                            1168041427264484640,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11842194058626225407,
                            6693138488663942165,
                            9693479840472939808,
                            6557766515945332492,
                            17527675102839235765,
                            1683781909006149565,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6732548018962826686,
                            886556176451583387,
                            1074481832211224653,
                            1672131685357546020,
                            13441358290175210535,
                            982329272818999435,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10391688221486231074,
                            9045549196032969733,
                            9444276653435874426,
                            6225312622280940322,
                            541558567715002424,
                            745968211499641484,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12256806308523389866,
                            17531109129597277602,
                            15744046266434284801,
                            6358392850927869028,
                            8469414460933919507,
                            991351303461708033,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9883294061007381841,
                            2250779174019482262,
                            10628018332261470086,
                            5917624016444952832,
                            2049519592332484588,
                            379266076439362854,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9613078049267231442,
                            498002071805226876,
                            8894979186094872042,
                            3998139538429512555,
                            5678599008406874796,
                            1590878292733900636,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4950488905387863282,
                            9262554117179726455,
                            4921783018946894254,
                            17490603013569449815,
                            10209295587593749530,
                            1372763087957478961,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10788537566316853227,
                            6995845539765443609,
                            104664623115210233,
                            15907011503294258444,
                            8248858225433775710,
                            725566341458236355,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16944071049938325933,
                            4749728484611508220,
                            3405682516230518655,
                            6820475010781060602,
                            4269119022583474318,
                            1674821274156141564,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12871477738709567183,
                            18114200996132079755,
                            3592273059302378510,
                            4274385300990581274,
                            15706293305947464558,
                            501709999276534525,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12199343707705719479,
                            2067874046362595679,
                            11343781441327819959,
                            13168117941744802399,
                            18433469638266699705,
                            1106934153178093081,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2016184029140046031,
                            4745925294113021416,
                            14995511775427280633,
                            6576761812152300568,
                            12445804865351333693,
                            293585514246953069,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8240525097910977200,
                            3259925884454885677,
                            17004190596641004861,
                            6619601534433055667,
                            1303374522468985550,
                            374110802427833040,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13123795113832111202,
                            10903337672960158845,
                            6752723252270236171,
                            15385624631052190500,
                            17180877953006621744,
                            65287946125841389,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15009197299648262080,
                            15152532429797765178,
                            2815893965498385907,
                            15067959306656178812,
                            5232020304559539739,
                            785753815524395898,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17095723910618273309,
                            2744427301118095065,
                            14985660426165692345,
                            388267362491534868,
                            1951228479964912270,
                            1853839396735533641,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7835975411447361234,
                            15330174279654532250,
                            16579546370766982770,
                            4622870926255475903,
                            3073957253741719192,
                            1112641646961527620,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13860100220934755379,
                            13520324235152712038,
                            11883708575721294398,
                            9991568485814358385,
                            677725614634631872,
                            1126118962078021306,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11499776424769742665,
                            16585142277311604800,
                            14311893717630360190,
                            12499414477178173879,
                            1793876467309852771,
                            349624445597804114,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5330992631153768577,
                            9989914648078003742,
                            1089841577571042965,
                            8914014067818143551,
                            2909897287550795756,
                            785114038819398806,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5980218324405144602,
                            13199639146447309469,
                            3249734338140408388,
                            8521519871948674408,
                            16869863831438094466,
                            311451473261492764,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14193854149569035691,
                            15592218319700985734,
                            1118172703185492076,
                            13962837710719904303,
                            3481513995647488017,
                            5555461062698585,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17140359368830257541,
                            12219092160540932474,
                            10175966493170977471,
                            9319671628081490335,
                            3226672351599190705,
                            1535418975942490705,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2633683222881274269,
                            9888177501527202341,
                            2976223767762395931,
                            5295657603404850028,
                            16810526736426584311,
                            1405306708564948191,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7179403267861074296,
                            1156194225048593750,
                            8828349436066461292,
                            4987677177394640899,
                            2049695865359047480,
                            826989428879384857,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6926115536228234606,
                            14910577069091588075,
                            14981131966152447847,
                            275474284321153493,
                            15781296685970183236,
                            1789796523757160037,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12692276720975281900,
                            6133847828666720601,
                            12430285534558099330,
                            13984936875185584626,
                            13683051119454372201,
                            301048052841276500,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2263556948701160789,
                            6115207373767876401,
                            7302222749673621563,
                            12890416865650336760,
                            8038753364971042704,
                            586577839001341330,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3076375082100382272,
                            18381862358333263000,
                            3704717733053235629,
                            9794863899981739344,
                            1780017009724646864,
                            1307789440947569153,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10004152265876561984,
                            1458889045563305829,
                            14977673244935805061,
                            5079233733829455623,
                            11760788465618343111,
                            1325782974090110449,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15005162335548602425,
                            4075140056952092001,
                            890651084991501470,
                            18418375515674345474,
                            7673090901082564732,
                            174811740617162466,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17005189149998607779,
                            2992805595006172062,
                            2376683054094833195,
                            4382688220844843590,
                            12186305317459798164,
                            1413426213444408571,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10504002673697582283,
                            9875146671074071328,
                            15206067664747058826,
                            17109141567575492682,
                            17592708120717401189,
                            589301336782756043,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7023210589813028872,
                            13191765606136660679,
                            11885709030959691635,
                            2586761658864977902,
                            6660360225713727182,
                            1203341243882119415,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13962806274771030849,
                            10645453697420098490,
                            16620666410056734066,
                            317993812922399689,
                            2740614137658037610,
                            1275878344426542241,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3800238653848732661,
                            15788066054738852840,
                            8681815587758963923,
                            12772471614953776278,
                            12617513865878340215,
                            400462204023618241,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12422688686876592562,
                            444617691100750151,
                            2560675910545817134,
                            13825073708261832206,
                            145023412912123543,
                            687751397148637545,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12385340442256480147,
                            18402759026464935950,
                            14410097002459109443,
                            13359793119432902303,
                            7729208089221831500,
                            421244296035777510,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8976533507926277539,
                            5076835256084069957,
                            10199499832690286313,
                            12619024391917016216,
                            6688284471354212875,
                            1565897490196708033,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16944141547361876579,
                            17844652383138083124,
                            4590478891019257237,
                            6476374925301309128,
                            2314529645085556395,
                            1348927820634119780,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4848188196378943624,
                            11641365785961733237,
                            3012098003207781304,
                            9220596303511391824,
                            3631445523137966486,
                            76993409067627931,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6207417271162366008,
                            4452663769493026490,
                            1611668112678898927,
                            8446145569155801737,
                            18190735618726711437,
                            1276869507744283329,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12778255112431922,
                            16176084500405599767,
                            2868643516108470289,
                            11443980812138394645,
                            2753026671852978348,
                            1351709508346272822,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16851470738792520902,
                            12992578238356814599,
                            16820212661547977398,
                            10598900572360762276,
                            6117248566313151185,
                            1505001760047549398,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14493751740189060944,
                            15916689961859483202,
                            5592924189713318460,
                            12586159018748688366,
                            2888613619452935993,
                            1447254563222165336,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15431866192908345943,
                            1372783172961771817,
                            6674572231512474008,
                            9308213171561522250,
                            15427072822580302156,
                            1279930637327974235,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3398908414373313910,
                            6216117207260679482,
                            11313592760390994038,
                            9661978777319070626,
                            5801425888080213318,
                            1089835299089482360,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1905119043038952242,
                            2897754190327815208,
                            1049929628414664782,
                            15031372479766987477,
                            17663711878911480482,
                            742585923929433618,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5989214689653167843,
                            12145031826513037828,
                            5321535885330764260,
                            5943661315612235342,
                            13080598233627902454,
                            1540434271538229804,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            122129220909060707,
                            5571769125170612177,
                            11547924214523723819,
                            11470421657066671811,
                            3640605496514731554,
                            867431070789556968,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6872209918229270356,
                            14737383618417157836,
                            8904576422172451121,
                            1644798916007833818,
                            14303734679146464716,
                            1097516396118306872,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2644820217271059863,
                            8485819857955007523,
                            4025886232340677781,
                            7961205313348692225,
                            11621983444639442638,
                            986424711828919247,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13757761388845863018,
                            3796983939469684253,
                            12090162670358861055,
                            5284444560707444163,
                            13872707122716148809,
                            1213998116539541653,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2513980284374294945,
                            11059070606052829771,
                            1277658812578202074,
                            259774801948244512,
                            3367663047947356945,
                            1152141701221422368,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12243994789027294240,
                            14073019371948447402,
                            3882936601498092061,
                            16776529504017295476,
                            8027522066371639486,
                            690918294383465536,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5418489366577403231,
                            3863060464893788931,
                            7263597997238078770,
                            11091099530908853018,
                            8021382689732109067,
                            2088942255581629,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14786113624313481203,
                            17553901309180685759,
                            12718762238529784308,
                            3362507727287492114,
                            11919539286756310241,
                            310840627542802285,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10978795069833424023,
                            15116409608641240593,
                            17114242115747884033,
                            11145374302586162971,
                            4050008846143743358,
                            948228268018750093,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1328510643944488801,
                            3450517460629097645,
                            11202375602229541881,
                            5995800240058980143,
                            2654760623864469605,
                            1501856129732976449,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11477544067061118285,
                            9340772296732937287,
                            9804854662207177080,
                            7230326152282267427,
                            14276192154178624067,
                            1588766550947596725,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18356635563801031232,
                            11959555258681758188,
                            14980917299752346569,
                            17603029786895496867,
                            3345670279167891227,
                            185718258393371739,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9445284682255788276,
                            2071563192205068850,
                            9799010855753346498,
                            7169307765271253967,
                            15293260213035514256,
                            1459071553085501236,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7203929278814369378,
                            12875706433761421308,
                            13910112743568305943,
                            5764140797609047320,
                            5096864215259319589,
                            115451314101814237,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12780125722711637927,
                            13519418790751874478,
                            13810382498987249678,
                            13589538434234429792,
                            18437054612875469721,
                            1474259165360360713,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15289647063488254856,
                            14878021736967188543,
                            9379542578386465184,
                            14864943218776741037,
                            156384284512009681,
                            249471107516534100,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9237531673089623465,
                            847556548915631356,
                            1076356927959502406,
                            6652421818521841385,
                            3816271507244105114,
                            1115283707760879599,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
        ],
        infinity: false,
    };

// tau*G2 (the fixed KZG trusted-setup point) with its BLS12-381 Miller-loop
// coefficients precomputed at compile time, mirroring PREPARED_G2_GENERATOR
// above. verify_kzg_proof would otherwise run the full G2Prepared
// precomputation on G2_BY_TAU_POINT on every call. Two variants because the
// underlying Fp representation differs between the host build (6 u64 limbs)
// and the proving build (8 u64 limbs, delegation-aligned).
// Regenerate with the gen_const helpers (see git history of this file).
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    test
)))]
pub const PREPARED_G2_BY_TAU:
    <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Prepared =
    crate::bls12_381::curves::G2PreparedNoAlloc {
        ell_coeffs: [
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11925556972146567016,
                            16334858588728237790,
                            13071673221206363140,
                            4783444548443525780,
                            9297645921556835221,
                            798161992579604048,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7376160959796979546,
                            11460070759966881008,
                            10492686378742475312,
                            18013447854818146093,
                            15456241908358726572,
                            635138503118167767,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6114330492017684964,
                            8790753945787631157,
                            9109704048932222599,
                            16063852289741162872,
                            8377119718655554252,
                            1832772230590679437,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2983431651187967594,
                            9420143361936114962,
                            18125422127679645388,
                            3852314742599414363,
                            11643442536871777939,
                            1101294227671161536,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2294565932855004968,
                            8712853091106804624,
                            314325015324907846,
                            2897586001882532972,
                            15351753497310318789,
                            240094519746241833,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14609329544893013314,
                            15552738521130673666,
                            365644675938667335,
                            2391623741463743042,
                            9436071031795180168,
                            1036865072251795898,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7723343621718374829,
                            10690066398185861339,
                            2494964356889654226,
                            5255070301620754644,
                            1712667504987536399,
                            1745829113203828461,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4632718803837617420,
                            2504526965494693108,
                            15967090077267071861,
                            16278734691332148616,
                            12688970467601551992,
                            1013120441263378432,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5851202926675066597,
                            8625259415380560030,
                            5775474712845252843,
                            2990323860552012375,
                            9396062353727999757,
                            331892216918616288,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14586558243183611968,
                            17524543797977087142,
                            147083720772250940,
                            2038665296479797963,
                            14487328477006156279,
                            1448680451683251830,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3706365046372996849,
                            15243608661071384470,
                            14635776042563515120,
                            1851701167321031407,
                            8820890296379460833,
                            522151934952124336,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13019291832814359394,
                            12555641349657668780,
                            12415858299046873171,
                            15175952227944801875,
                            3992626979404397404,
                            844381687493518199,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11126498769479087478,
                            8805449472259600475,
                            1510354522944665755,
                            17808941009963723098,
                            11328606327463895348,
                            1596419105081771476,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5867919863234581894,
                            3476735551422891727,
                            10424079148189177212,
                            14369416176525130752,
                            868294977286103665,
                            122208983685177601,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7572668876069719148,
                            2330973559498381271,
                            13545139541393905918,
                            5725567809532437965,
                            16621707573129851746,
                            711162580003806297,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3843478665137236538,
                            3675084615728424722,
                            8564678526025159121,
                            16999961491484719438,
                            3273021535376930499,
                            974015536801244301,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9123377991879708471,
                            9616151698894714241,
                            12313169436103337212,
                            947127648629502427,
                            8690483684094594957,
                            1542679077341447563,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2130326851293644007,
                            1864996701564738455,
                            8914823939469460989,
                            15925830120212279554,
                            14989069730935700779,
                            827449668066551955,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16458892502348306983,
                            7137808065319900475,
                            13439136086791834271,
                            724299830792069365,
                            10224640249104169120,
                            546184773436092812,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4518717432653601243,
                            14492929032961939219,
                            18284070141478398033,
                            17252523880739695892,
                            2205770993354539260,
                            231153303040177112,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4955655540166178043,
                            14675133973170698999,
                            17596569191572679049,
                            10600808513070253868,
                            8186661412975786009,
                            948332965792059826,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4852844119935032131,
                            9935075551820148163,
                            5867351718971333704,
                            3936086317971208791,
                            18275735868563066676,
                            545789397746215247,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2447373873837925436,
                            1234312995491994788,
                            14292493294204475088,
                            3902654513106541673,
                            16388187768823863606,
                            1592570474472269559,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7744705746090013493,
                            8722998669371208313,
                            2955259259686435374,
                            18404147149789984156,
                            7510161283664275675,
                            1776633548580548608,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10644212501011758444,
                            18174418352105425130,
                            9026101209493494077,
                            9381211259926198700,
                            2441433427485537878,
                            26062380204325124,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17085134777386197337,
                            15881017453795797895,
                            2240648804981151702,
                            18260351677438622324,
                            15947898179225436361,
                            1315838886102656678,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11257329424145065763,
                            13690459336946258764,
                            6695064597969666423,
                            12778832381962286288,
                            10515171397961685936,
                            381672177004480860,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4611385146047256996,
                            4052754049362437872,
                            16989457981807047355,
                            3936797166479366182,
                            5192545408015349402,
                            1435774512547819891,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7084047558779944701,
                            9626909257903347559,
                            9250987082530970809,
                            8801078100550715265,
                            9534484070328760150,
                            493548359054297234,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7254161078027797018,
                            13369984146035489217,
                            9616842106413199363,
                            2184292869118097084,
                            8607566966869653221,
                            568750848593883747,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15901971784311991931,
                            5756597955549135338,
                            9700260701042775840,
                            12741574641154939916,
                            10178634458464376840,
                            220826804012950222,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9468702904214068391,
                            6239078212046684928,
                            7161867955362909256,
                            17785989680722892250,
                            9865574747861867555,
                            1467277188581815213,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9958127659275201304,
                            15360861477991874964,
                            10123924482064440108,
                            17157333375013097794,
                            17937608932770869332,
                            1867001237015344352,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13968176466994135366,
                            5093473826063865659,
                            16513647053870722007,
                            5475981439086306460,
                            4545503042193042684,
                            1357221527254200606,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12930537219024873660,
                            7410900145155317948,
                            13282912267232063476,
                            1255727418175152446,
                            10175289421165377116,
                            860359874967180614,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5740162068019236338,
                            6205377666921376127,
                            4456010104034385689,
                            13012099264919685927,
                            8641203089394412342,
                            1372006608689547444,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9365256325042561559,
                            13473824556215842601,
                            7463735299289272782,
                            2119773591165422221,
                            11205871340214357749,
                            1137077072718624618,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4676781693728288546,
                            14647568887065035792,
                            14132140483406663355,
                            7825231661290057599,
                            819261293088754039,
                            864210717551415263,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6896426462585699730,
                            7279525779059087447,
                            2089668438770731545,
                            18430234912904148214,
                            18164228272439466795,
                            589025176250196430,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8066904534413120515,
                            11107773763295503095,
                            3165373730399334189,
                            16106587365370867847,
                            17488310015311944708,
                            40306167018761091,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14429102156842674961,
                            13739501427883300657,
                            11807842210337743782,
                            4483918912619648466,
                            17108837848864884999,
                            909446742166143374,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11461109822967215999,
                            3439733053692423973,
                            4153822966198804506,
                            2407657612757065990,
                            15215346696393807937,
                            809335811619583418,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12577792596589933665,
                            3636397963681197451,
                            8979642198983157713,
                            6467061689619318210,
                            10925018415394213633,
                            1019532484417326479,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2231885201267778989,
                            13406771189559235338,
                            18072141183735622269,
                            1487129837709726924,
                            10875962354990901662,
                            1209010187589422702,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7984821080738966426,
                            5719856785783239740,
                            8860597701099210262,
                            4920863900918890593,
                            14258364950512669019,
                            125340966872087079,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5128805738020324751,
                            16476916576174314996,
                            1442452774975542713,
                            6368352327765800700,
                            18094350238656923546,
                            427969321588608447,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11792183570448155801,
                            17848119796283727434,
                            11155557430152273618,
                            3334742214622074146,
                            15000064684649871598,
                            1182625415207740038,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11111526835812777551,
                            16380875126660333655,
                            6014265187412799351,
                            5315157657971051331,
                            6220127417582054131,
                            415169338083420016,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8890457561652514281,
                            16336197849867252356,
                            10696724146064442375,
                            2645705216965123909,
                            10223881345641528178,
                            1222543204380134143,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7191170992571602861,
                            12213494161358985122,
                            9866037464796415042,
                            3705934112773374605,
                            3016385753016521557,
                            385921809431968270,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16482954637898166134,
                            1141341707790941028,
                            9602402554324474750,
                            10185604952446518074,
                            9633585981084043879,
                            518819547267629340,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17965195730078322788,
                            12392394544316727267,
                            15032903271394778541,
                            4546801778556919804,
                            6261655323304072004,
                            673135665866127203,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8071415009007423084,
                            14216763050342008883,
                            4506692598771651778,
                            7528017385186778962,
                            13719211296604378125,
                            393738973054281436,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16427720866586134236,
                            15212373498309329420,
                            3643649688635862081,
                            8594002229243870298,
                            164069860849775524,
                            726311772985105951,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13646025530356321354,
                            4805203030097177961,
                            4477965191493873002,
                            2313118110565637423,
                            3761505352942693568,
                            1230120882559557730,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8998926740203506483,
                            15880038642362105915,
                            6268407000256145714,
                            5191782250763503027,
                            14237573602258474383,
                            484639140884232455,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9943111243254682744,
                            18318472255882475324,
                            13571450170926787394,
                            14853165478676020188,
                            17950516708013693330,
                            163001116655971720,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4304890601088043684,
                            13138991095591798559,
                            7377387516030395154,
                            10384181943284817078,
                            5013288565839709822,
                            287515778618826920,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16789189218846744468,
                            17591211352416738358,
                            7071502329548331428,
                            18001296795702038384,
                            10542670614720193692,
                            1812313695472264355,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1347313782638110912,
                            15563827670095966574,
                            8528222503263621715,
                            13798604610752650701,
                            6778837199264737085,
                            1011848413758700356,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            734817677876110503,
                            5854585256264582086,
                            334138430196872045,
                            6155601865791876810,
                            2652393225856490844,
                            375815759663713446,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16002147226574056424,
                            8053237224882802450,
                            3236670577204687712,
                            10210865371723962891,
                            13459509575287652401,
                            182570599403682143,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15898094034384353080,
                            6828080128498380283,
                            13545830842118254139,
                            12068103860928432101,
                            14173220372352601116,
                            1694111816045768161,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14366471346686951181,
                            4287153257986750142,
                            7986456117959934364,
                            9743385975877592747,
                            3388044839083390602,
                            96118368847976389,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13503863679036200327,
                            16547648953825637398,
                            6554048617768648459,
                            2144949389763425056,
                            11594343720407951953,
                            776898610673756742,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14655058322303047459,
                            12663630452867052329,
                            1607552166811780890,
                            13199456528985121131,
                            16009453028392312220,
                            1583990678679838955,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3255682889253732767,
                            16205722982681261314,
                            16685470358175725853,
                            8135237604720195462,
                            979465975292346401,
                            149726446797001823,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6317867241370351506,
                            6989105149313190394,
                            254401568534383422,
                            16085593705782647103,
                            2441584840233537761,
                            1470620779431431230,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10654996161109209561,
                            4913008374494045075,
                            830110283099472914,
                            237822853392351616,
                            6994949963944583279,
                            1508575888759217721,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            713338152734649787,
                            7626101469040709255,
                            2357858527393622267,
                            12851775181363330680,
                            2411035513330019007,
                            1536818054420239111,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11674810961136445161,
                            5139751116306511815,
                            17279289070027552053,
                            15222118387818977486,
                            2494153841375211675,
                            1154756602320974811,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15769897178124930931,
                            5545549438802126241,
                            12879995836034519648,
                            138496503735704387,
                            3701535853118223166,
                            1505742657035701761,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6049997757183208497,
                            6671192561513207057,
                            6156609390514919058,
                            6661866985712923436,
                            2699320262523097042,
                            1318394725772967427,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15757531954383357344,
                            6452627642910992532,
                            10533357798412181683,
                            1683449058595576534,
                            8769742629188678161,
                            299304671647571100,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4793668539016172811,
                            2415615960148147973,
                            10869456137131108372,
                            3801431787896192347,
                            6404957811218845892,
                            357451575432191537,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5244776855537895858,
                            3559593276311949714,
                            3398526756267480135,
                            174704202399055038,
                            8893836775364690083,
                            412040673102691610,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6743700563728528136,
                            15729429880143749511,
                            15261868516776415466,
                            2739872274899953210,
                            13667849105787592420,
                            1548726575338815177,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7743286655838215909,
                            18287732019564225306,
                            2000839571169973336,
                            10226041126781700750,
                            16519421059711517496,
                            1222945639384908193,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15092742031842063585,
                            16088538989226631047,
                            6476916101235260243,
                            1956121245703582516,
                            10272058724981952145,
                            430125553764659967,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9543467134579083332,
                            10277368540605392780,
                            6155278220592138770,
                            4384131266727665742,
                            16502566287356048644,
                            932245934753468159,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            728277409143107400,
                            8315848048817740892,
                            10779485302711982151,
                            7673811698248762249,
                            10756447624130217429,
                            1780297371298501716,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3707189436230605035,
                            7092300099250693560,
                            10053211640856464379,
                            4451179339382987962,
                            18188183595215930878,
                            965629733302617885,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9643260728059233000,
                            17698395208997584959,
                            17651180530927236801,
                            16495451099362550455,
                            16437577553700238695,
                            905809579651933199,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10762806880803719376,
                            9665840686566496027,
                            10378124170845148828,
                            14108407175350111053,
                            12466649603547076768,
                            74187771592352373,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            894390426989420211,
                            4915756262105781167,
                            17773407678371676077,
                            3677094084260779856,
                            12269604497980481256,
                            1673756327118024326,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15419269349106340257,
                            4590684996808076231,
                            7902584297827962278,
                            2298794303208685784,
                            2915523884566138853,
                            975518849555379370,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14729264152624729315,
                            13842421227595851236,
                            12149390552619137911,
                            2932442935058299725,
                            2572471806680179211,
                            1301179682580977864,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3602320274031637123,
                            4549120751730316390,
                            4577751040996779773,
                            5666007386798490733,
                            12701363054406401199,
                            950346392340607557,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3757798579998322393,
                            2687057174873694421,
                            8106286444258661411,
                            9565319758626399782,
                            7101854685347134470,
                            1066341188734641281,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9411974306085432162,
                            1876364169309533710,
                            9297244742336531561,
                            11790550411004924425,
                            110420279322888889,
                            1574835900168092609,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10135465715582669924,
                            5888745249316319324,
                            12574012923763850223,
                            15394457956171293448,
                            13487207971264497529,
                            323366419069305926,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11808084758017513997,
                            17542145097896666197,
                            15254852154039001416,
                            11172876529594247583,
                            82804573089129725,
                            1100393435994641122,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15792334261488091260,
                            15664370882096496682,
                            4328129064570109832,
                            13746153841324600255,
                            14622444386512042336,
                            1542240897425236768,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7114969781701025149,
                            15209578178090951305,
                            11910278753167397379,
                            7659299309910282074,
                            1049834275295697081,
                            105773786702422980,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16333009619424460127,
                            18402257492326244105,
                            15722177048311106963,
                            9678282164515557545,
                            5656878813561048989,
                            1711388897655013024,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14419915349325497910,
                            7625217460656094447,
                            12953389866437708099,
                            9737339247479167946,
                            15844059670003310514,
                            466280760752221564,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1809434212726303843,
                            3323875763088362209,
                            919357733044024897,
                            17249207329359921221,
                            4596458360269380961,
                            1609225490152881277,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15517659136916550441,
                            5696113384295266439,
                            6857964258985848377,
                            809627535603824413,
                            15000318224151708581,
                            749390806081368898,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9332939628460013116,
                            1417093594997992969,
                            4551946947395501966,
                            17396404858012074431,
                            3029564293180914811,
                            46078006863161939,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12117462210517030216,
                            13929813358380075838,
                            1596015817236698829,
                            10717529643351790357,
                            14283104359177705962,
                            327742863933271186,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16115216343004804320,
                            12233630647181341097,
                            3367436550829686603,
                            14623987983350015804,
                            4059474184591876732,
                            1377949123976948430,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9451746805276553127,
                            16609774532777155534,
                            8768608242852924296,
                            14784492530604263809,
                            9741514413170068570,
                            1659181642242133777,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9537393270191716250,
                            1397355729486116855,
                            7125016768837026989,
                            5210859394566944699,
                            12704010402184440659,
                            210085248337053704,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13893676821391894812,
                            16749098412694357001,
                            14081751926009191920,
                            13201158536951229592,
                            12094178806090527711,
                            517437589336507406,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            378743727516648407,
                            17648351188309641422,
                            12975926215877150136,
                            7573815506508353945,
                            7318633672851800605,
                            838708338386916095,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8858671833410818061,
                            15734057103885603263,
                            8174559681880847722,
                            11826193153290957965,
                            10993918273381808872,
                            1176305779383038052,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12154549900415474711,
                            1110750699660043559,
                            8573796403527537913,
                            12451163387896206633,
                            3909307498221762975,
                            516122164281208898,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5651259372124568485,
                            471466572676895219,
                            9302049074256594549,
                            10759297508168777835,
                            13934905161862610669,
                            337339592633982467,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6528374147929617120,
                            1769785864558557995,
                            15932425088751386866,
                            4681122383350660232,
                            4075851899407991734,
                            1482977009103686343,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5183286319329772906,
                            176418030640625046,
                            1501009277750541029,
                            5089933868902208430,
                            15072711914685130685,
                            1441588862219951874,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14069476943542182780,
                            17609119497167353850,
                            2832964331925873964,
                            16495327919870442377,
                            12030482305250964049,
                            1675925929427531265,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            529291349917868811,
                            12470120313203347870,
                            8880490905569495252,
                            4110309824621609055,
                            10436410552337467311,
                            944379703348586496,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10556168310153542276,
                            6113228765361567268,
                            16130009544633491333,
                            14648361234202272355,
                            14042717501470532053,
                            996492786119551541,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9512523011052657084,
                            12444813521601129359,
                            4700186509217862140,
                            11374964666032411519,
                            56099546866910421,
                            323145433017646840,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16962970681580352142,
                            16471161318579800395,
                            1018964678812838772,
                            8682068031693775993,
                            3448400605368972042,
                            640697735462612161,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3560619757227365473,
                            9239243271928898510,
                            16851756891694495662,
                            5468673555917513062,
                            8700786422073143531,
                            839552925230056061,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9131748704496754548,
                            7652964773799709102,
                            938714332355521978,
                            6746697541081000237,
                            11803113115771606090,
                            469738253972182323,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13063655576566652596,
                            14718358522606014344,
                            11006294343158719876,
                            5070073490566833786,
                            4243541238020681130,
                            1421320762938252499,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16616633371891427580,
                            13926270884137701317,
                            8865780346621894920,
                            17045906590149271031,
                            4004874915172225880,
                            229965255933045386,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15727020333284892723,
                            1408618811925835037,
                            2160400779427370507,
                            9084422006724154754,
                            7465610955758401799,
                            1111916939878300581,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4231063385204006468,
                            6692286242515971765,
                            12840106154291826465,
                            6308434671736959326,
                            7099587447396526880,
                            1028029982669721856,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13571610410760125990,
                            14018032570565574502,
                            3821321449206415206,
                            12045681115513281696,
                            11715798535000231907,
                            596498659151257747,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3161165881830155608,
                            14621547560863010622,
                            586642549319917417,
                            12621313459726468753,
                            15611155550499377360,
                            1472073315688039675,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17123945709495407279,
                            983080166914913308,
                            10548359586401616571,
                            199684120836799127,
                            16260990210560624856,
                            1687917881382393005,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15361311713497454850,
                            8791504742806747856,
                            11841949218505484641,
                            6857553999143553898,
                            12614022801591747341,
                            1159458459360487396,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1746232706613982183,
                            9321968290137173626,
                            3147531215437443169,
                            7495822192674426705,
                            5085699126872187752,
                            1202378474173246663,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14109292319323709941,
                            8837164345633341015,
                            16085914088913335172,
                            10606145229660013815,
                            13136333598762987795,
                            117030878975697651,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            206015072447997228,
                            6678084232404658769,
                            15512433192324750573,
                            1947246275073001113,
                            11868218730342628702,
                            898343431753349968,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13243949749461136856,
                            1456768495750214448,
                            5392853117218405090,
                            5149890048152948429,
                            11721680857179789157,
                            1635388107631303391,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3973267250237951866,
                            5441388041636904961,
                            2688227262880175287,
                            15054480845827152347,
                            17247725248213860602,
                            352239917168993980,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1207728109026388598,
                            14476710012185150693,
                            2246780294016035650,
                            8040152720630273383,
                            15441010369445169050,
                            1561817579327666993,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8938946251427727247,
                            16597471624015905641,
                            14777521400799040529,
                            7389997781945013160,
                            10625730081073723070,
                            449069910363110226,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14558829160461254718,
                            14079950709367841150,
                            1065784927039347426,
                            10196180361325571431,
                            14632448736227742601,
                            551543384160592678,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16401361693757855006,
                            9111384123277549069,
                            5415179535502693821,
                            16761090842230868438,
                            7065149778710016402,
                            326700431859901376,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5161187698881313462,
                            15407135823601577666,
                            9270750016512848597,
                            10831961254719748753,
                            16235285677107722958,
                            1314353825375987945,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15286911181595032076,
                            11405526654630506187,
                            1603227792067920441,
                            3208894901895109568,
                            15495411639620801511,
                            1040470953499882009,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5468435024657977853,
                            13247469680060670109,
                            10385145302207955786,
                            16230964911083068008,
                            5017235141787163367,
                            1481852877526385886,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10660705633345303159,
                            1994969116635853755,
                            13562983942719930317,
                            9645646741213478491,
                            18049781344630597594,
                            452266371163308633,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7571661775478230929,
                            14551836743519661867,
                            4622799633818304510,
                            9245186152632899517,
                            11012794527344968194,
                            512736858921116536,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13481694380377457528,
                            16685885696606617168,
                            14742797850045547646,
                            2126962111213475867,
                            17734367877784460856,
                            887544097761840258,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17525520458437974424,
                            7911687325048473898,
                            417532147825699589,
                            15677672547644698745,
                            14087139669875924684,
                            1752586674539455933,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13977551202433501289,
                            12984642978132260751,
                            1350456832944518575,
                            12097599113612379988,
                            16803922891152180597,
                            273066620017501672,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6297405972153664055,
                            12902774946485910994,
                            13138230030164868088,
                            10780926650763746832,
                            6797769461669410875,
                            1238457993593915504,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16904503979535321440,
                            1408910392891853049,
                            6037142385028834161,
                            3431984774353581496,
                            9599630797197640472,
                            1412728157748839413,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7038865756066255017,
                            12299452990964825911,
                            17010321623530923447,
                            8786134161077542211,
                            18365359851027786169,
                            709647176188727409,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5290708680087885228,
                            5983300260214320404,
                            2354736656674922373,
                            8154193338493432181,
                            7352054854753352652,
                            807112183606431831,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4527180954955170283,
                            15317809688666540922,
                            5248754679387965240,
                            3078313981575136394,
                            4723741570854940471,
                            1683438614474169462,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1902414411508588832,
                            2530297999347974788,
                            14749003414645755159,
                            15475221928703011698,
                            10663716132128396108,
                            1297988066892568624,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6614613281375555752,
                            1282743761923657323,
                            7043923039857527261,
                            12732999155888226651,
                            12154503124826837585,
                            1209217381524218249,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16549819809840464555,
                            2843763347348832098,
                            2813125057126697496,
                            15720883435851133298,
                            18271778644183848559,
                            1204286554256504400,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7109130842929914832,
                            1803790392705807387,
                            1271608437981737847,
                            15682633792013818447,
                            10287229682566749616,
                            665117965630175262,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16677421006981650064,
                            1353809749177007179,
                            2823006776672705092,
                            11997889663167429847,
                            13645198407115909447,
                            579219906856826597,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3295392738214448860,
                            602025461241339599,
                            10529090071170925240,
                            11014396106566593324,
                            3281859532403750380,
                            1023686700067908406,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4333070397982432537,
                            15271261348859370469,
                            367511550370076042,
                            1595173370854984586,
                            14823737049126087984,
                            123706930642833768,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17067910549732339510,
                            2675228548236584427,
                            3366984011790424261,
                            18293407976473074349,
                            4277244937192700671,
                            1360653507603512731,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10502157757803091650,
                            1136729077429196483,
                            2817952920149324208,
                            12462078344733826288,
                            16855115728600786653,
                            476775998752184792,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12466547238993612874,
                            14230232247552762226,
                            11032148330356353138,
                            17388341393312774378,
                            15962291913680472096,
                            769364543640477477,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9156594793750494078,
                            720099626785901314,
                            12159788476925010600,
                            8164616588957315705,
                            17483683783295944096,
                            1128501590156391302,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13568472953403430530,
                            10425567279225319957,
                            2858702026807031377,
                            17794196568706119115,
                            15029146524323265425,
                            1206824831631949963,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6027184781054211025,
                            3567124591125006918,
                            1455754456895461908,
                            11267114916138278309,
                            15366536353659424417,
                            894651456579924381,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9882683328822627495,
                            6233588290637950484,
                            12394612088406285135,
                            13269965119810265843,
                            2988463812925714940,
                            466135280360634319,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4671417466836588458,
                            12446023543066084485,
                            5835227541829011504,
                            4541155711362429371,
                            15011857688172476804,
                            111210992209802193,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10748952635887004411,
                            10593762535732814301,
                            3870727810516816661,
                            15935339202481202298,
                            482208230438523923,
                            1537447471243287219,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16478427816050564964,
                            13951178306110306370,
                            7821910697691839364,
                            15627073127185310532,
                            18133150159137840830,
                            320598979090218648,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5655057475721088625,
                            7152016465095975941,
                            5136724542486956931,
                            11510652293959864250,
                            12352084350816523931,
                            86459393876730119,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8167098127403838789,
                            10009544728165019089,
                            5281780780238363612,
                            5254545207112128533,
                            5291041501809772749,
                            1345794758377513160,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15407935611479727828,
                            17444373291685559913,
                            14858046868961498084,
                            10480498280602340126,
                            14799578371379936965,
                            1799770797561505150,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10383963341046486936,
                            6151654411419230251,
                            1446668829327413838,
                            8661930182144193523,
                            9460038173457368098,
                            1021728507160749981,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10732344871601567081,
                            4334673536361391169,
                            9275192152153963178,
                            16963048303554692871,
                            10497321073870628943,
                            1490316745275747933,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13368228803805399833,
                            233648479810044396,
                            1744316522751782240,
                            15636772238980171602,
                            10383665931125690719,
                            1607563114181844858,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6568085714437292113,
                            5101175589341546760,
                            7591833363917562526,
                            5836411255444437623,
                            15288032628559525278,
                            1610706357942007232,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6001630566030395757,
                            16129805276494871555,
                            7523417683147092141,
                            8776323131469443086,
                            17674263472204926407,
                            1380557021907819413,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8772970160173574340,
                            7637752877799773755,
                            17297763445606480487,
                            6659181028059823153,
                            12421196189321658209,
                            98602328712329484,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16553051718223743050,
                            3218562186620828586,
                            9833137852221944360,
                            2667369746885895424,
                            12028931757995944033,
                            950757411665912337,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12030030694574518889,
                            7931542603921300582,
                            101816645790800048,
                            106839734525290882,
                            18123056086791568206,
                            866172480240768937,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4299582374421754980,
                            4911522814358395170,
                            10792876498511164968,
                            4050684776833885301,
                            13536722109816584856,
                            1264687921304412012,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14253402898794693558,
                            18439357004671279245,
                            9051386666342325800,
                            1891210518001668833,
                            11509003499379517097,
                            610832256088481259,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14308615142097129630,
                            16623326968740845687,
                            1059676555700863543,
                            16218903140657921475,
                            18334609715041820650,
                            1562981183403800777,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14265831947459931709,
                            12936849133223139951,
                            14443276418286039564,
                            10247674956864454809,
                            9261461096719451295,
                            382971898310213951,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13422982461916310958,
                            5791945023783326227,
                            11766378699369540778,
                            6735874070020392856,
                            16867841614520325699,
                            378073988924542713,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            568080847528580033,
                            6408357865125127946,
                            4718860532948024976,
                            5285675181274058692,
                            4455822686012410810,
                            999791150999693474,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14902341354191165434,
                            2399981349484744670,
                            14741612180701683271,
                            9503973564303477071,
                            12643047586985358574,
                            1643142375783486922,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12987337560242163967,
                            2966937302222962418,
                            5364312612470833228,
                            13678303015294802026,
                            3699261205278052722,
                            1298154652391170703,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13647962696821389635,
                            4647655972148652231,
                            6974525783005341187,
                            3347156329238840650,
                            6353963009490271060,
                            709413528475114695,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5766442711547784631,
                            18292044916531246746,
                            7748107104524064152,
                            10324299033448649756,
                            17742860873905350030,
                            932989510525917724,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5732814184726408498,
                            1905669038607155493,
                            1725234534676840196,
                            3058629682070606784,
                            5405636245563067879,
                            694655108436057901,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12004215899791765488,
                            1173386383280046821,
                            9955519611908558965,
                            9761017236938582347,
                            9840737489470457618,
                            1751633212192667902,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8535274538249285941,
                            18255874256303527824,
                            7209800330484923578,
                            3027483556817002669,
                            820495281422851882,
                            148776460129766445,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7749143497653962735,
                            14031753154473303346,
                            7809437872328726922,
                            4560378531568944635,
                            15018703218703821543,
                            261996199556258592,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11076649602625253382,
                            15583953263696522455,
                            10203388756471500152,
                            9600420485263200137,
                            1071738251141265722,
                            700544702725449380,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15790206933076334079,
                            10297086256199083434,
                            15804807926673601075,
                            4833373735191513687,
                            12636865756548857487,
                            776231900821580851,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15334298496890743428,
                            2826685152164360990,
                            7175470919265348253,
                            5425665915173631694,
                            14632536189457243836,
                            1701022387535361452,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15541316952703968885,
                            11160997080806785272,
                            8746539955128385463,
                            625747746950577198,
                            4077951635570616541,
                            1471092185855862049,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5897846697653356645,
                            9321351151729585993,
                            7568852200058501855,
                            1804085864876831337,
                            11333356783842732127,
                            1330582394599085149,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12197497649925822766,
                            3029833677875758847,
                            9305838143291299092,
                            7597018948308741834,
                            16973480339130745518,
                            1133387070872371892,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14593563103302307974,
                            1829201435110481791,
                            16130184464017805513,
                            14560666697122324786,
                            17428144041703903970,
                            1073390042513349807,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14414903075805128189,
                            8902469104767411198,
                            4695729346012608323,
                            9565817119317933834,
                            4714977099061070373,
                            34275625141951727,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11575551943331378410,
                            2672507706477613786,
                            10416791481905448063,
                            15494766665438410851,
                            16704237863789430000,
                            308150495559285040,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14283382659346721287,
                            42530497902050799,
                            15390820371306482716,
                            17627590674407664722,
                            18298292587364709056,
                            1004858058192201254,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15593892003601633031,
                            2077932155293169495,
                            1522184259765659779,
                            11804207129545949702,
                            9078354854076132430,
                            692429891431248534,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4478047225735513661,
                            13411898304025400022,
                            6755740283454682689,
                            6000322998287227664,
                            9754130746044496211,
                            1280528449755599608,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            984476016254060401,
                            2431754274653309244,
                            7754577478243921170,
                            12880305955497648400,
                            15055166514941213761,
                            838623968196294350,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            257091101910803328,
                            7883021547568789660,
                            12589824078918633284,
                            17135247941195385925,
                            15674723618642751547,
                            1436010198166488565,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2117094966451289265,
                            484853749082556615,
                            12099182347415633329,
                            10433803614192845641,
                            9259015132199125866,
                            919051356237729194,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11675661611922998596,
                            10686683891376836409,
                            10649728010168079581,
                            6005217101701744108,
                            2284559317054356761,
                            431825079182241213,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15402226273880853799,
                            6696707583683680890,
                            13843409310781041729,
                            17324948297966165536,
                            9488539404567146743,
                            1719619610782356041,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7740694143082407503,
                            6492825187388212285,
                            9650884700497170049,
                            10168146683153001151,
                            18402117195135816663,
                            541775821806158532,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17582826872222881437,
                            2419822064210418524,
                            286980701277818449,
                            4471651494654395060,
                            5706430364051348955,
                            1335064285581651911,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16834149116444330620,
                            10066281195196696187,
                            12390560865895847540,
                            5082375909949764136,
                            4551273775178877262,
                            1190241473559022355,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10277476091594738129,
                            4769063059564626034,
                            13998782556996979165,
                            650516747165815470,
                            6486168155632149180,
                            1778522175311639524,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11350040998889391055,
                            8741209360794628970,
                            3760761206803157263,
                            4164028918244472551,
                            9922965434023860770,
                            958535836360016344,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5645982535784659398,
                            3259879960073694328,
                            4818173533411024297,
                            6989662266862452190,
                            15616552747090197116,
                            1483047994247159276,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6093156432645773383,
                            5321349270571331022,
                            8746127340911382677,
                            4003371938802359523,
                            5045751805194305670,
                            547292343156303991,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7402510376603328352,
                            13268803879435769365,
                            7578622836652749115,
                            8876911470440150777,
                            13412896860529519539,
                            551667771941000527,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12438582273312932419,
                            16842491264131298795,
                            14092745956873588624,
                            6442722621031209966,
                            5294941136156335032,
                            653632333364076808,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3649067416659357,
                            16566164308750056516,
                            15853625786235262764,
                            11985555263714426603,
                            8446413818196758461,
                            1007597375591640764,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9134530811377558452,
                            10743273482383861598,
                            6003511514509806997,
                            4974166568332422256,
                            10040325193874578021,
                            1156935684280498729,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10791658693579402575,
                            13806017823607500567,
                            6894145059311003048,
                            10595062118108192384,
                            17576731900753995157,
                            1234398446434964638,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13447386888886297167,
                            18439482024947418550,
                            2451274134805851798,
                            10400855478581261058,
                            8411936957719574558,
                            428495666531752333,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9236824069501882906,
                            1305176338002587906,
                            12895116492717863092,
                            1022148700742803578,
                            10665444195044054698,
                            8854770771357378,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            855549409580140714,
                            15384072111494308871,
                            14900855716610383648,
                            18414070532024735541,
                            1995871966295721341,
                            962918627183374713,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15930356490932892387,
                            2315819748477261679,
                            8448726659242626688,
                            15532405289547574449,
                            13303151266315856433,
                            484812929629932941,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14706104739967909176,
                            7328988080856465483,
                            12152197807570747914,
                            4935687581244095209,
                            4500829633481699227,
                            1272180142451776014,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7982195887344653672,
                            10199301322289390962,
                            12852509751386764296,
                            4405014795784432800,
                            1421573529248407225,
                            730165052592497303,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6148276544739069826,
                            200898987545952172,
                            7329066224807584416,
                            6814208882660762165,
                            11270524602974582531,
                            1122774707915077937,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5063801738640317349,
                            12163016638935551861,
                            8244346646936683651,
                            3264563404852292506,
                            9032795079598396400,
                            1122099347997592482,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10356345323289111159,
                            16950739724928955967,
                            11103534038032159228,
                            13965664353190328059,
                            10100036684770641184,
                            1729016462296915556,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17307851119994987692,
                            1221354035216668064,
                            15140895173796432824,
                            13236691990190486308,
                            14239194363794974421,
                            609053241222380109,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6708668983795643944,
                            329849456319455276,
                            8859301544212381332,
                            8423689021633516986,
                            5392638759356677795,
                            1445140635026565048,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14981688028507330186,
                            18416860402212265487,
                            15705858966326119583,
                            17394853409661734436,
                            10013805154136700281,
                            745068460821009326,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8991478696616932473,
                            7405567657118568546,
                            11776595659247412236,
                            2652420477662958979,
                            6126292633064641998,
                            508899619119953282,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16278430403796657036,
                            3512389402488507324,
                            8522323250054531412,
                            7503432881349374948,
                            8854603850894434951,
                            718757310017528748,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6463605063516530148,
                            11316774844684830737,
                            7604983210990288548,
                            11693318139774632409,
                            13752270205179779553,
                            134177615803577934,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11586550250770058033,
                            13396651550621967113,
                            894824845114436760,
                            12256276494114898444,
                            5828443320809628173,
                            427808104210664979,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16798953408681408750,
                            10486478302611091351,
                            14763511786583104597,
                            14443348201406372252,
                            16026553302000184502,
                            1222765188098221098,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9888771659460338876,
                            17499150657474902220,
                            4876241848200880028,
                            18375553134608775321,
                            6237962292133196721,
                            1000023303178807068,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12451115650411409360,
                            4720047205640314474,
                            9131235393603010651,
                            17178767008906572156,
                            5338719232870238327,
                            325825136352100971,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5206554609916517299,
                            4788105876976337453,
                            16178942110654878503,
                            7512537901304220635,
                            8287547278454499397,
                            1206475975924514656,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17162012041350376318,
                            6596276686994201943,
                            10035858116207546753,
                            15459680986883647576,
                            11953804491592686982,
                            64624429952148489,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7244578673560695435,
                            14774925784527111673,
                            3139989656966554203,
                            14409958096347625139,
                            6162974383380312991,
                            787381125083898041,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            964376610664718148,
                            17905905576048319289,
                            13887044201204775612,
                            13240783434975279039,
                            2091855244573505818,
                            595805578109848180,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14197117873176997436,
                            17699835358721724855,
                            16338678874568709841,
                            7823775105559628562,
                            5523110817520296745,
                            790396415935820694,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14677488522377884755,
                            12258932925743408650,
                            10016052490406587741,
                            18398290098649741259,
                            15462818823084337489,
                            1233217459840653084,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7591999615884527548,
                            18075223874460873643,
                            6684714364703593520,
                            14662462173734567430,
                            11930202137675577378,
                            765975871438094116,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15471201705039400309,
                            3455001272911433511,
                            7973550011377549681,
                            4095731330693508195,
                            6658201555331874357,
                            909606139064288151,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9816550220121165754,
                            2463528024718824214,
                            6421902019950259771,
                            9938773670540370117,
                            3431246167526572269,
                            1617962201769260112,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9857092290450040357,
                            6446251589387434650,
                            8500788398331470638,
                            11247867793881600724,
                            15351112529316923967,
                            470615213801508513,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11848847478068824019,
                            15953193637118012864,
                            6642406716687539943,
                            13682072572904438113,
                            12168646851410753558,
                            660585667364292820,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5195864754440114583,
                            14717989569474406588,
                            764013234210717328,
                            7460283496121946334,
                            10360087603789119755,
                            354707759774127734,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7283007813424736303,
                            14698149941513445542,
                            5419861808116010362,
                            4869141838988423694,
                            2565472000233618703,
                            499011439464952642,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9620852391959830978,
                            11797267328051158786,
                            16849350027406478471,
                            2042679277934559074,
                            15649143285186293510,
                            1769660967008521564,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3474443786719318305,
                            13895979291839746103,
                            13503327681116120987,
                            8352521575920348064,
                            4899554656393003458,
                            127318890018687026,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9740156985040527726,
                            14062519936970733147,
                            5935950652366044751,
                            17259881294645464631,
                            7648168830526439805,
                            1313876073053663622,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6077792753366829399,
                            15814872151667598022,
                            5881630449629336968,
                            2340328437602471916,
                            1329029073324277560,
                            179096513479161672,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11691107325823545188,
                            3951943650021198707,
                            3004084068414263872,
                            16177149676890026442,
                            545025422486732044,
                            373130671928335613,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14436485247562212145,
                            14390055785079722082,
                            6802110372354206318,
                            16054961682617713437,
                            8291693710065610287,
                            460133088284168333,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15609894277874790830,
                            3520877652784040202,
                            16889355961098636722,
                            13428200175552106151,
                            4405081207892889146,
                            730827225125088990,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7683270448196653420,
                            965436757788688520,
                            5624465021819746026,
                            8556217125032881652,
                            5565431464602624887,
                            100070634253081236,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6730370609936033294,
                            18230382748432162571,
                            13969116106781629372,
                            9246842910486474949,
                            250907748553802583,
                            677551781897198955,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6229008888401616029,
                            6301999709520045790,
                            10844063232126285971,
                            3875008105431359871,
                            17602636867466643287,
                            1748199371629067432,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18019562834591848403,
                            15445540857426442977,
                            1262876873371340907,
                            14930248245065870240,
                            5813676975993321716,
                            1228361843338730421,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9154888051995123482,
                            11793583978376683526,
                            14155003584632594933,
                            17100458224243617658,
                            9466735011427984669,
                            1573070905980026530,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2789924502477989988,
                            1789805223858413848,
                            1292265366005939739,
                            14932001982744470614,
                            12058586488036579126,
                            502241590254173281,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1760117820111699695,
                            8806335062100170015,
                            7969864783681429809,
                            3263926016537925248,
                            12982346120433958748,
                            423804028004090838,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1617383170528027962,
                            6720128694741609344,
                            13638550289413158828,
                            15587030654054476679,
                            6157979994712108220,
                            148250245627288425,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1564877982363645952,
                            14980429569400212009,
                            9817677317336117744,
                            7092343800806901058,
                            8283769966364960000,
                            337354760694218127,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18236771844017101081,
                            2027844651191072111,
                            8067314733565541887,
                            15873326412431961469,
                            10975003867493323403,
                            161205323180982695,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1906755284805073794,
                            1573543250341168977,
                            11221009663296529581,
                            15308037042778060131,
                            3397692999072696152,
                            775134971754681931,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15113901146194846714,
                            4753818568876326387,
                            18276725408831568933,
                            1420752961294252962,
                            15237402137953730112,
                            1540839359007499459,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7617021285281298138,
                            12351790618547599267,
                            15198736351846765325,
                            14276494012955162928,
                            16250023746087810082,
                            38630579497506633,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9644967813107739153,
                            6685462290336533045,
                            8524733862507764860,
                            12940096764591469181,
                            12385684114319791140,
                            1025697457303688653,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11106304733771437209,
                            1969847532000629640,
                            13557867517137585998,
                            13587320034588603358,
                            8640931887011803461,
                            1518179492371639071,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2604064966728506527,
                            6922241088526427662,
                            13462436602954247859,
                            13902988014958064733,
                            15944190895403533825,
                            852871283734068303,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1959521018691567996,
                            3071933930264841997,
                            10819956055610031722,
                            10198975020982422947,
                            13306756698879195840,
                            354277697405995235,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2905815252924093418,
                            4804612582505997119,
                            7832772939198209269,
                            11666263092077937670,
                            10794921094467513417,
                            992217338248509696,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7772088985534276417,
                            18008902553255892813,
                            3727504315264615681,
                            294163955478609274,
                            8917445582098125211,
                            471347578296707255,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6906814344118258259,
                            11889188607612695029,
                            9305070740297416538,
                            15074722697990317176,
                            6116560248409091163,
                            1831544946012720252,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12653720154581985076,
                            16784494276165199171,
                            4621511277319125862,
                            3460459809000322303,
                            15688782065787574795,
                            1802464951454614029,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17242796131169191713,
                            8917904062924712457,
                            4555598213891193139,
                            15595654696059543635,
                            1315360184078117865,
                            138210125766055574,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15354582944054503961,
                            7317394003156761504,
                            7319782037230520281,
                            17038608445604340532,
                            7021778851102260068,
                            245313859056525632,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9628570159603646819,
                            6646982870404769129,
                            12601143354358445599,
                            9967928343784489728,
                            9213154740704683486,
                            842080761970843494,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11171253505777851014,
                            14251689094357063877,
                            6420970113759501168,
                            11545162438443094787,
                            2910578451333658017,
                            1481107841370450217,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10334545241538730777,
                            12710596646710957529,
                            6533867814023423066,
                            13710105843226245206,
                            3362464128605720166,
                            526116607093607704,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3479343087439599832,
                            8995353238416155858,
                            2352857322168379713,
                            14358886336930181561,
                            2882816849039941350,
                            65282257514800343,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            693975797316198830,
                            3216817924022043647,
                            6783621138782180451,
                            9229998978382190633,
                            13817701674833300799,
                            404014521367538414,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2631181834796279890,
                            6865465244614601451,
                            17252136365302676587,
                            12862400938923291835,
                            5009644284695612685,
                            166275576348799099,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11771817948593270070,
                            17051819898149434979,
                            4651519519412105420,
                            15687339730920273267,
                            6522757887572037526,
                            116192281375660481,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2260184264528173778,
                            10210787862957987037,
                            5867832714893279665,
                            3637168092714147709,
                            15978506368345710456,
                            1825465386532779562,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18187183006672127841,
                            15720481874882462599,
                            9794208433577921835,
                            11707935485459927674,
                            10950170517229711882,
                            1417423657790619462,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17052088819402401794,
                            213187642644976780,
                            1566455377200019844,
                            10267856563392647696,
                            3850829850103395684,
                            1516081778816594409,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13309462842643256329,
                            8745328686258936320,
                            2138077766523380040,
                            13609941151911358085,
                            13110158721747718099,
                            843136711938873032,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3628575059070082902,
                            5375562782356704418,
                            15292822915148574734,
                            7357477466615566511,
                            11470097017019735639,
                            575678184745249416,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5800101597459162187,
                            12897919661554396175,
                            5749018385061640798,
                            4973656397069680698,
                            13716183325370422894,
                            1583674049482086203,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6333586568831189090,
                            6857820192021945499,
                            14834330572356798546,
                            6456021078102150762,
                            18046342692640677075,
                            1603559214115201550,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11689689939579325663,
                            11524733982066878886,
                            14106333322073883248,
                            1369672476919584106,
                            5303831343735155145,
                            1815540583824087056,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6870629436957946676,
                            8395593508713549324,
                            5227624762738955341,
                            14472231088637639239,
                            18245476472116663505,
                            677607514594141188,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1759238432782850804,
                            14148320875021722375,
                            161156132222717039,
                            104704321651436098,
                            10852416503715883073,
                            1078808954456871014,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8002890436442123161,
                            4707101301849745190,
                            7502855836593894425,
                            2063358170362279118,
                            1109004709534639634,
                            913600046300823696,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7745237912110697675,
                            4568048020342522995,
                            6827833430308072742,
                            3969409800068017743,
                            9363562798800420577,
                            1712621992524464364,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            503733034548025058,
                            1224161582931573184,
                            10813708582833491543,
                            13966786455092580102,
                            9748872238365194426,
                            410042249140929369,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11574098333606378763,
                            6151438856288008040,
                            1295852700522942520,
                            2480475359445764733,
                            2111496808227083529,
                            364257510085786748,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4890663561164336212,
                            6982661080163290211,
                            1408815182583458273,
                            7777116839910071313,
                            12732205372306621194,
                            1612730721922604476,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12271107747833409169,
                            17903244240046918779,
                            16941321200990957765,
                            4657788724433299824,
                            6386794740662264765,
                            1801352275058639484,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1856016738856337889,
                            4913877744720570242,
                            15436782956713450199,
                            10382644047489435419,
                            8943072923259388842,
                            870034538836119805,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15403838252298797488,
                            1770571998306181896,
                            2493324830803603003,
                            2389040677011876755,
                            6149917141661798131,
                            812872656500164213,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16986351607259536291,
                            6432911767354423774,
                            11067544335167525263,
                            4169538258448236833,
                            17066567244911162322,
                            271057970575896220,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16673466327403992069,
                            8728811735723560212,
                            17316992312888886968,
                            13171528736499246323,
                            4306680176197053045,
                            666655519655859676,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1576568291874371903,
                            5674265165640857418,
                            11655223367693189788,
                            7608576459283826141,
                            2100647013342745848,
                            1082028654992544356,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14614724378806630097,
                            1211385095296391493,
                            3372346757588805839,
                            10120413208437550071,
                            14657655015256657365,
                            1245925135878060710,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8048756261548760061,
                            3752326594358919206,
                            7785367383808924560,
                            6895867163714182022,
                            7712718917190978635,
                            785804053985723491,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9935478279955039797,
                            6824993079362036886,
                            8474168743214332635,
                            16283744502545759966,
                            4563586482932533308,
                            1441135089347584616,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3717366999113342264,
                            8715459760807463484,
                            3169992774274832445,
                            6998437019195679477,
                            8932487288973081945,
                            1851108503345162438,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7231661550374710895,
                            4346473901003033216,
                            6025258602223109440,
                            5000161988292269240,
                            2426665970001413168,
                            1473774334147908245,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14885591206670401588,
                            7669225956167260167,
                            4204367430279692624,
                            12557328967124909068,
                            17964420834779509475,
                            856009350598164592,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5592788438311016733,
                            8539495963324453842,
                            15174676731382858195,
                            15184144502380514546,
                            15405138936132558623,
                            1592204956221888645,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1428144629205055718,
                            14309613097265020352,
                            6218531945239583359,
                            2613570837696211461,
                            15956831273077321572,
                            613688129859031185,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17724459694961980044,
                            1712687769489594330,
                            7176341114697117557,
                            17544354775463871602,
                            90189646883834560,
                            1833621169398558972,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9024162772283717526,
                            1756273913574908891,
                            2221117933523715276,
                            11531703024810712619,
                            15340309474866999163,
                            1138474323323125470,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3347536680028821906,
                            11000339386775869064,
                            10088246964071034140,
                            15451838769483404774,
                            9852012535391158801,
                            1600911439501110386,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1889742963113312087,
                            12395130136904023340,
                            15079324122845583450,
                            5213238838767409819,
                            17480637030792910868,
                            1048088086678376766,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3967695283270242628,
                            17314619670179671838,
                            9412560239054202087,
                            299232575354085940,
                            5994364371205128721,
                            105188487112917482,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15711480746501693560,
                            644620387137773212,
                            11413043432702558063,
                            9133591798898543386,
                            4714688826404300983,
                            734895993419826543,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10016143352503192912,
                            12056594488974465489,
                            10690119421400085407,
                            6063061598035504228,
                            18131803956496134921,
                            330285622802132447,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8176494531631940641,
                            3595017461147257851,
                            99450318881478714,
                            2722272514917504019,
                            3480322516442732220,
                            1562400622865690590,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2239141410284766695,
                            1252379756188326153,
                            15015276588277667986,
                            7017858399445707837,
                            11589255690585861507,
                            24403915446220137,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12142707544617236018,
                            13424914098329006047,
                            12948508768865320148,
                            6126051138541539569,
                            4653308918464384708,
                            509905904864260497,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11074469526781604125,
                            17835779313400850281,
                            13016894874127377800,
                            2750720492476478164,
                            9798706872956081920,
                            1462689213914250047,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            657823640975140288,
                            9081943589268120471,
                            12185298188276038795,
                            10131394558727255803,
                            14646672380985019787,
                            41121574443442097,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13780581423017258895,
                            12112106572845675511,
                            7199128607260094097,
                            13123762757086778277,
                            18380350472118354916,
                            1450763121436993636,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2168295911937557577,
                            9417927238004244394,
                            9038204375940530146,
                            3641775054100273448,
                            14706844732328537620,
                            539431996302168642,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4811445544608193624,
                            8908432936072753304,
                            3910843857381672728,
                            17837696035146587574,
                            2691568608182271399,
                            1746937004146828591,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3779468776893813160,
                            15113507542579561546,
                            5647755718044925043,
                            12784918757261412810,
                            17570984365615050899,
                            99033306831554368,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16695773489227624055,
                            13454214442513659800,
                            13718160544872091267,
                            5448583397800426656,
                            8077231275047511552,
                            978381129161214731,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9788396266923176742,
                            6246759682444571246,
                            5797872549001865462,
                            2833887941380116127,
                            12897153641356681461,
                            102092579119955864,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17452801798817584766,
                            8250921907676490894,
                            7024091394224743796,
                            17490302059595655412,
                            12817158211012880197,
                            797526746071361221,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10627710397289688813,
                            2145038833685487687,
                            10115520745936693014,
                            13604262030508249937,
                            5995166391442006699,
                            192913778019192436,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1038614807573142718,
                            2339529296952016484,
                            4216937922333943838,
                            13736764648592262165,
                            6922026429426666523,
                            569031779721435531,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1146094057598018040,
                            6058936810588890758,
                            3494736087099947236,
                            13983677117266594632,
                            5933596276314286058,
                            473649367739216809,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3380951182420483659,
                            1354234010036100486,
                            14059274565711145033,
                            11432533891532624594,
                            5939977328433094167,
                            1276541122996103506,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13630406259103455851,
                            8406508743301047285,
                            9028170919419454349,
                            11359579923001526623,
                            5076078439152849498,
                            1542524018236372624,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6524815426517626267,
                            9628473004726583872,
                            12961036077053706218,
                            8720303087634976491,
                            1625745956205284954,
                            245210428302159529,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16014167616761426267,
                            11391023543046801469,
                            8477294296535933807,
                            17140181195246614544,
                            4992338115617028834,
                            234463410569124982,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10702346424655560519,
                            6973534236428619603,
                            2132447154126148428,
                            7884138117132204986,
                            16738848807302566801,
                            238591324268840391,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13495778902260784748,
                            12674641593491168722,
                            15238190638477862417,
                            5091852271291966328,
                            12887187546193535546,
                            465577747828933929,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6132818351947590565,
                            5224239168541998153,
                            9364827248415327367,
                            7500465297502493303,
                            15683258250742973249,
                            1736689139982775664,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12619349705358827933,
                            11589986684012701599,
                            8478358343294014181,
                            14150336504331880937,
                            1098909976801765368,
                            1837944017979616175,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9295925073123906153,
                            16789948123543045087,
                            9992995173666932493,
                            10384186267964862469,
                            189092933295164691,
                            928421216747693343,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5657227261502783884,
                            10481575992825755575,
                            12852534117680265352,
                            6551390174100456647,
                            2861733810370052715,
                            616973362766585847,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4726257432775496252,
                            18282342031798436986,
                            8888804775890718024,
                            5727139763788355103,
                            10636595209552455173,
                            109181679635951316,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1468675434059786388,
                            9013679820600335613,
                            198551958820584871,
                            520240110933810562,
                            13932903522520032071,
                            15498170066324210,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            903617550355405586,
                            10882033145266524563,
                            14992513647901952441,
                            10748253053672168457,
                            11393295244825418940,
                            188549837416623849,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14149360224756068038,
                            10118893560323355021,
                            89962033538707349,
                            5038818549706589299,
                            17485136638700572415,
                            18661879015767733,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15543947721599611528,
                            6514437771861411141,
                            18278234815917518609,
                            7506190055397539751,
                            13259247767143916349,
                            1267365088274497062,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13817272620911592145,
                            3959217330069749302,
                            14110685520752818295,
                            17999954551793050606,
                            14922300306742255299,
                            274465417703529089,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            15398533160777794603,
                            8988724776762614424,
                            867523300488023544,
                            10383613868885390134,
                            4001366584719817008,
                            1421627386999959757,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3404121002601250642,
                            643715100164083383,
                            10357726059280565892,
                            11947460431038854076,
                            16007756234291862893,
                            321885045600372225,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12813210219520704993,
                            1147998089278713958,
                            7423644060386155257,
                            5300110017972969495,
                            14430431292957282483,
                            556837840738587866,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4901749189344106900,
                            12842680847425107242,
                            18049575134012410863,
                            14334924552243050742,
                            4831874013491645317,
                            1231736870337211725,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18385481721768868622,
                            14366108526485732405,
                            10033931138508181194,
                            18248134782010508977,
                            12524226921007405283,
                            1504999656186464664,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2921674558598841225,
                            8149617458354309328,
                            1994766075555320964,
                            5059609041575062079,
                            11509320826069736403,
                            1563838037813555461,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            408179174090127167,
                            2310198664916344449,
                            13623630568683445440,
                            1225479829839464461,
                            1314211268778660850,
                            98115091526803905,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16844638163573555693,
                            596358288822251692,
                            3197558008374948453,
                            5343908631482809806,
                            4819092800530130076,
                            149506110552740081,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12099026007114374188,
                            17549248948593877427,
                            9851405802010023164,
                            9818509176288490327,
                            16672862849878518738,
                            1537701962298020243,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3573509671664851199,
                            7297653006721172559,
                            14141267901476864826,
                            11184589881444202325,
                            14631835788022587814,
                            1071875822122882262,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17301229731300534455,
                            11334943526650262539,
                            427798726286065318,
                            12405369455114056537,
                            5944707807112146633,
                            1716053518123094536,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5174312471639608792,
                            13589055377058869589,
                            1389558569579741012,
                            16187965611024249573,
                            18072377302636297603,
                            1757870565843133525,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10799799951639595154,
                            16237111341652665737,
                            5164736259233629738,
                            2246011626603226390,
                            10023699186773610821,
                            1317939572584929167,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4204941000402455561,
                            1256891490954464229,
                            16647219675294149416,
                            9342534687073572843,
                            5617263917922806560,
                            301768037496288370,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1309184372418157388,
                            839417882516872731,
                            11452553845728022649,
                            10959458418503596755,
                            13954201371421618646,
                            1815671464480448808,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            2655236239667186542,
                            7259251914163771429,
                            6932269439847718805,
                            7014061408234067279,
                            1711888469007624341,
                            1616457193800027410,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9037264112046879551,
                            15547670444712607929,
                            13677391531115667061,
                            10415445572415629737,
                            5089363296566237467,
                            32537888183217779,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17039155197893239304,
                            15808380855740269416,
                            14246697385100382652,
                            5121749173244302614,
                            14281549853660958908,
                            853741022092446672,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8611808659443045587,
                            18389054823498408955,
                            596030688651975355,
                            10486438957989503657,
                            6990539099961680544,
                            1470256765658782558,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16935735538129886967,
                            3914586943009327996,
                            6974658085656322958,
                            16348659526546713116,
                            10004441471175197605,
                            288285004897277149,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6004689603552514733,
                            13687783477445567893,
                            9065325517762943974,
                            3153594263269357662,
                            11883830541493721035,
                            1603080297547140678,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17250016866135545774,
                            15359686955786944565,
                            12771401794703437522,
                            4870732488407493073,
                            8086462307128859778,
                            830425717566023706,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12472474932509282500,
                            5015254865519534055,
                            1046532302408555644,
                            9088557284198436700,
                            13605605617688226452,
                            912615786447090476,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            16164031551506548205,
                            11115460024409552460,
                            9650047394105950371,
                            3885859036376147041,
                            3179795826723439296,
                            800276339049225864,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            1952257428944937061,
                            4765933621196023621,
                            4354548780225760155,
                            7183148180993179018,
                            788802990981038856,
                            1513312483938989143,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9189539231792771724,
                            15452682771193780034,
                            4460941027996314770,
                            4676757286723579016,
                            13952147609359274381,
                            699578579415700671,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11729111138800370692,
                            16459314413374052088,
                            8174432340604483462,
                            2457989749254560923,
                            10917110052085036639,
                            1259603197583414186,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            18225274810136733185,
                            12946438279241636953,
                            17015819716612461141,
                            2190767887779724861,
                            7427758516457844662,
                            266545412094517055,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4134843357627639107,
                            3614189885401481216,
                            2985246351220113184,
                            3214404360499202928,
                            17966045811405645424,
                            1161707924831675554,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3505862282315702665,
                            6928139799962796090,
                            11088259690874814376,
                            3948529839947695732,
                            8340353925297545557,
                            1766915652075343978,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10526687703230050682,
                            2608628397398007043,
                            15046483330140355434,
                            1512483995284632460,
                            6281705785584325064,
                            882377842414631025,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13733701579950713834,
                            15443500586259955404,
                            5020405524050314196,
                            17609811499297800802,
                            10686190099098444404,
                            907935720499953393,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5219497437650863647,
                            8914910322466332856,
                            6960264873503194952,
                            7521337706427535055,
                            10435449137177110054,
                            1855116562948665153,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3947971213813819225,
                            3985672343359261800,
                            15741925706263554293,
                            3490655800486633623,
                            7347579991141980421,
                            1697568492355534089,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14173238338997795675,
                            6435235678700531995,
                            4809134439838286924,
                            8352431182993115427,
                            11819769208528215913,
                            1672011249579288265,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            5090581916949263251,
                            17322283451937994092,
                            15175209780312723017,
                            12241231124103099608,
                            13172170630554217583,
                            31343410208816024,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10590929945557649397,
                            13389094510321847133,
                            13921777874409872497,
                            7119002176226726347,
                            14955859283587817705,
                            1761614462693490606,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            17375925468298353919,
                            9652898321685399945,
                            13816741675743216756,
                            11603114438153812563,
                            3295208075794502531,
                            32635347862028275,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8028750343586139848,
                            11449484256635855255,
                            15461282071678093591,
                            10925220089703344411,
                            10130279599246422101,
                            1817195929208381583,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12601926062286917072,
                            15899078716378871585,
                            8724664095645831093,
                            10027043763850929713,
                            8269797299854171227,
                            839256083651064962,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            8760797105365171336,
                            4652595850720728706,
                            7750461893121410942,
                            2329717933672960504,
                            845496713747931444,
                            446861739887541797,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            4918229137218181462,
                            7507157771599521247,
                            13897537939032697259,
                            9251210598584951262,
                            5128879514976160436,
                            420275362641386338,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10934448794607816686,
                            15366671333045242822,
                            2450156526228210548,
                            3532091855015075767,
                            12158079228444849968,
                            219623913380634838,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            10601273475020169674,
                            3267923388208515397,
                            14067532599840757145,
                            8755646368691085684,
                            12648232626657575013,
                            781173788210717509,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            7195934384692936389,
                            9264503063949455592,
                            11490921913245392651,
                            14473107412237585735,
                            15085398434996602579,
                            1097789344695160382,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            13144905322119668681,
                            338926503536273694,
                            7334715624773456723,
                            2341349602276477687,
                            6389403802860322939,
                            916951698347559100,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14713195902227473990,
                            11340350047156062896,
                            7184957490715138390,
                            11467712195203988715,
                            9035846137737188599,
                            331924350185729697,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14091021038604436676,
                            3579886937146050202,
                            6462782730510095848,
                            16070072743477008021,
                            3171416459613772883,
                            1413674921736990226,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            3836202597954322206,
                            8767674938116416773,
                            5998032352453840054,
                            436828849444488567,
                            2651737937588967897,
                            196843457547691190,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            11775585960971368653,
                            4944570035884304478,
                            9777081632668553221,
                            12532206013378397289,
                            10416077636982140675,
                            237967815125606091,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            12796144687285966258,
                            16792907346337183841,
                            1748634166801502867,
                            2770416065004473031,
                            8684566957770253743,
                            1125763907515840288,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            6965197344523728656,
                            10032376769352613873,
                            16857111790620621062,
                            3003609568677781698,
                            4093808801510054481,
                            402747877809757681,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            14224243587048502267,
                            279303328981674604,
                            18341715871466323221,
                            5891052763451618193,
                            13837721834073534283,
                            1730729440054882025,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff::fields::models::Fp(
                        crate::BigInt([
                            9265166481282088648,
                            14011484668677791233,
                            911572026762428283,
                            6585497528024296942,
                            13215815476591079485,
                            950707015315105415,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
        ],
        infinity: false,
    };

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    feature = "proving",
    test
))]
pub const PREPARED_G2_BY_TAU:
    <crate::bls12_381::curves::Bls12_381 as crate::ark_ec::pairing::Pairing>::G2Prepared =
    crate::bls12_381::curves::G2PreparedNoAlloc {
        ell_coeffs: [
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18224921920243037867,
                            16963770909527695949,
                            5750666909137969540,
                            6161224692404551343,
                            1779667657432873794,
                            876975598140104716,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17031392770447353241,
                            8347243996717321168,
                            13637856449354377907,
                            10779456231282798217,
                            6524819496748122126,
                            133672546881804795,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11169460466474763443,
                            16541113849791986472,
                            18157258669354112740,
                            4670277874380365461,
                            6308897172577454413,
                            1453219972434381642,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16915316941324633080,
                            6929210588322574971,
                            8046845609944923742,
                            13465826404040537395,
                            14641472736206511944,
                            218517007424029694,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17004275010030791416,
                            6886288035289633932,
                            693632534468802136,
                            5643457850259105219,
                            11821411702346265098,
                            607627877932214575,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6089656385827115660,
                            5791103514104218182,
                            4078729188458198728,
                            4145928541233359833,
                            3749521653594911683,
                            23976174838776978,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3484290394869898944,
                            3340633585033722815,
                            17930800035169753263,
                            11941031283403997603,
                            3511686102450541365,
                            244400625670965891,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3400249205652249471,
                            3605260308113300151,
                            942696137966592067,
                            3798810728325294958,
                            13689108681756147758,
                            155709277517970750,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2250827024878690431,
                            9816057176282534816,
                            8712803972187857374,
                            5899355345021079657,
                            13418479232299621997,
                            1293277701318389578,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12036006052099125838,
                            3700925562646565776,
                            345500549045550025,
                            17163839783288038704,
                            16372573073137346944,
                            57056869733269883,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11570007658818551920,
                            10048697446723432352,
                            9879427060604544296,
                            10768308492827554173,
                            16656027119278124576,
                            1203632325751657597,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8907419218176342511,
                            1776012211739587451,
                            1311228790963909606,
                            6553009295796878527,
                            6588715355770690975,
                            779661024965383477,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3407047717810488313,
                            1141378490915682439,
                            14723159434454964377,
                            9428307792150386838,
                            17667317324284165043,
                            1388012089122892252,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8206737354163542306,
                            2747085725749573616,
                            7274874395313749161,
                            4321691458098418690,
                            14988036829348695092,
                            153807182245114242,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10205474264356380278,
                            5648951909883922848,
                            5901882852471479563,
                            11270578652756219415,
                            16596020704212016247,
                            1551468028598773958,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6902345645543514969,
                            2233830121602397560,
                            7163053857775157969,
                            1161568851134297062,
                            1775328417239519623,
                            655077872367247919,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4423995686279571207,
                            5430375670809537692,
                            322690540765201676,
                            1111617052000379964,
                            17827150657938423071,
                            1154083982071071472,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4939342530227888869,
                            4809682087106630566,
                            1159719011802309246,
                            18232720220921513326,
                            6664002055928931026,
                            1427410198545954895,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5124694146960789200,
                            10188786392370950951,
                            1467013830598429745,
                            12578632130507870628,
                            3192043900369628320,
                            1305277161335651780,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14085434351501177381,
                            5955665746621367979,
                            6491426391467322948,
                            3263017054051139443,
                            8083695236882769366,
                            360092074312304790,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12878554161521367434,
                            16004116564281596579,
                            5042908963784476677,
                            8018796535673409650,
                            16111840984345269468,
                            900678512631140660,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11037867144951181021,
                            14388547707444955431,
                            8442237624900298809,
                            11821709842616055192,
                            8716221447547051359,
                            1847698803686899312,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7754654138356729663,
                            12204548797848641278,
                            11696640774377778046,
                            2659147012723254477,
                            3239979536971372011,
                            1019497630109983832,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9644364304708729225,
                            3990130509994833012,
                            12909045453998044372,
                            14020619909950325632,
                            16209079635438457338,
                            703559621829185138,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            521570785793274175,
                            12404027862411124815,
                            4931117276198269118,
                            6406226738391172544,
                            16978164789443697382,
                            1466646186048471314,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6369444957486195873,
                            13401562769647497339,
                            1415577575641429197,
                            12950170093148885741,
                            7467537918588629961,
                            525265662562615178,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9683891409327803370,
                            10017959498877575697,
                            15738352953123636737,
                            9268535294623722973,
                            14134148843199533758,
                            854167626694229369,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5189756454962075004,
                            3501592988243799300,
                            14141210686218352921,
                            2412684742995210594,
                            4786102726184043015,
                            1358831899578570599,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2682124014904516618,
                            4244245301854816209,
                            12685457260915855580,
                            11785156823431467961,
                            4267864347976941081,
                            595813251103569173,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7364063293712008106,
                            12471029772158807800,
                            3342271630049838790,
                            17175780719102638159,
                            5913384578854058953,
                            221222134797533293,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7205993492005089978,
                            2036712367098471051,
                            7570607233112494568,
                            11569596427422099143,
                            6970857398857291165,
                            725176844204550684,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6005565266186309318,
                            12482572669684424062,
                            10262083292538430329,
                            10601152579822478562,
                            4099353575147380456,
                            869163636127439223,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6239783908246374097,
                            450045265194096615,
                            5606674444368488401,
                            1327537000889327356,
                            16084780994075496133,
                            1334500056675070690,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10605871278473531979,
                            9135984603060305114,
                            241270738292452010,
                            13419786466557394744,
                            3248981344493167606,
                            1710816401947983321,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16167974822407890865,
                            14689826144721018088,
                            10376987218136324220,
                            4583685378037727088,
                            3505472045211471485,
                            54627025631437628,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2050827882553462916,
                            594250610035665750,
                            2518043122942970377,
                            8894709905443930469,
                            1645766990524457905,
                            1106095359447071707,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3758501633341042599,
                            11462242976391931513,
                            6201412642550203359,
                            15504535530598067316,
                            2661355270434921423,
                            873272241935483950,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16401219585552266351,
                            18008461639933310933,
                            12217125206449652562,
                            5903520424482151120,
                            9923210668374318005,
                            1000477819626246033,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10975385657225696960,
                            11181651068654221137,
                            355847784908137714,
                            16902285089692119826,
                            9641272264320018245,
                            359824809922211466,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2851440656742385648,
                            1230619420857375851,
                            6155121985155583224,
                            1245108839174225610,
                            16046505728263529475,
                            1008221681564371387,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10479775368060405226,
                            10122840672322492284,
                            852619325199600261,
                            17666546167084609510,
                            4352255636663450137,
                            805107438418495538,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15797208644828685991,
                            11535302793342446510,
                            3049445466110554953,
                            16941303234103131061,
                            8350844141847619831,
                            1617772006564176342,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16715651649328085071,
                            7304417540693398222,
                            18095532323422457358,
                            7407399996396137861,
                            6699646023390499489,
                            1251570944214912321,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8530459539286371109,
                            496344283265439822,
                            15881954623470855999,
                            5713718989037209361,
                            17400785332830072828,
                            363183623536061292,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7941335846990478532,
                            12180159092814656818,
                            7071759709197086383,
                            6547266708116200308,
                            7997081955092525206,
                            1273490915442949146,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12284058898697354217,
                            4386892510910142952,
                            31329226496008718,
                            6044498179237592792,
                            1379524638040847638,
                            613295971451425540,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13827296025878007916,
                            12918839068372257559,
                            15967093291970752785,
                            17439067018441205825,
                            8842921037316501461,
                            272592691257701131,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7106682852896079949,
                            733279532362536949,
                            12600886512146702086,
                            14969070760735717722,
                            7479171185138273103,
                            932704770564453332,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9446771951302749932,
                            17428987259647436246,
                            4167057343882312664,
                            13391826912862829828,
                            7832075961701158755,
                            561586019676639032,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            316393679405781128,
                            5691069122067286704,
                            9935705868496101812,
                            9187441437135626577,
                            4301536559310963702,
                            256393869407154216,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8676386141610251463,
                            12661136323934043884,
                            11339766753545024048,
                            14028293925521533286,
                            18362600735863831926,
                            255459776059276708,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10615303616352915582,
                            15050438809462279235,
                            17557058219476473105,
                            11946224410177810956,
                            16281370447620096284,
                            425321681076965315,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10446212320761833329,
                            1026731385478323501,
                            2366764486811875264,
                            160189944832515513,
                            931781359680516116,
                            1859148571574098850,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7698460029191652095,
                            8615549064715288194,
                            10622381980563899295,
                            5571394233394763677,
                            1373825518465732172,
                            1263235425302582159,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6002676170175787067,
                            12121410416672258813,
                            40316775438886922,
                            8145007044074013318,
                            4556807643828526456,
                            161995735955794890,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7606980043778855591,
                            17937151583074055490,
                            9997772446709126371,
                            14698011731395524765,
                            12027625623578449187,
                            1287950432354042674,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8346936315609994790,
                            4995455257392546402,
                            18057400567885651792,
                            11906099631993096781,
                            8075565949645759187,
                            923804830269221345,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13966862128171710475,
                            15638883887559144612,
                            9453880202985090081,
                            12926818221768370737,
                            15372297807044008578,
                            874851818651331552,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10630882475075833403,
                            2949612633433729267,
                            17858855241800989363,
                            6303169227078002664,
                            5848056443139694547,
                            848424508126827256,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3395692183407230304,
                            3923069112423637350,
                            11776078573175767156,
                            10293444205909427703,
                            2273326497408843376,
                            858664959922176904,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14223288236053495135,
                            13640342854149663094,
                            6003554278130424936,
                            4767590822799549678,
                            14340532282828748053,
                            876026687154923953,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1172064007834759010,
                            1518886498482895630,
                            15681675251077189154,
                            1676668847148281686,
                            8816506638519127863,
                            792527051609122724,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17334764738295049702,
                            3950535710313686437,
                            18421983508673331442,
                            1646967177776227990,
                            10320811204666731981,
                            890667717965081435,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7011355965432919169,
                            12965003468038668775,
                            2236713308897809486,
                            9792564390237250842,
                            3476257900504351056,
                            896462779609780136,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17510675175101443227,
                            9607648012023711150,
                            16814541521559061456,
                            10513950899249779573,
                            14056131454840696700,
                            608974589780839287,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            68254755976021275,
                            13627039527070419308,
                            9294381047346897420,
                            13554535746101203258,
                            758305286261276233,
                            822785707106552219,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5558123840884964354,
                            22165123852183458,
                            3184855218011355486,
                            14222260621266892913,
                            9162075556927238727,
                            631069426745691298,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12180995559826214260,
                            13250749096872300826,
                            9512534291587183784,
                            13760915172672982679,
                            6091450227349793138,
                            1454166316785133972,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13848115304358180649,
                            8428755612591069885,
                            8976708833098079672,
                            11025346688129374462,
                            9053237835820407402,
                            23429796087735114,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            507259760432531851,
                            9437690467768972631,
                            13392836511327507697,
                            12186249868078985180,
                            9139905540625112533,
                            313078319678103995,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6712932192756402522,
                            1980406708206318167,
                            17737248225051827855,
                            12608782461731358859,
                            11044318293378582102,
                            400048704072855662,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11195258147990490807,
                            4333222269194020551,
                            8419854405712007370,
                            10791680155783604508,
                            8947477387950856462,
                            697751077534142571,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16566722484945411617,
                            11171775714982692723,
                            16793358444165111621,
                            10719221084320871479,
                            1536231240942106734,
                            1667553480017349150,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15250701489052305942,
                            1246439802665269425,
                            12530773988666316113,
                            10024295570021294474,
                            161780560200097458,
                            441474959492149743,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13698869803834151614,
                            1826511158127443596,
                            5154993029877197877,
                            2969842095770378548,
                            648954980105002236,
                            109966954703369541,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14799378493778239420,
                            12848298807796260062,
                            828537812591013043,
                            18139036503946395124,
                            13356088904638179908,
                            412675462396887820,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14153673796610603288,
                            1392957724067217551,
                            3408399685394106113,
                            1135750038257243562,
                            15095115932619168155,
                            1765313619571181001,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            868567606822016596,
                            2998478295119321540,
                            15512415946926356421,
                            15665801294270186304,
                            17080272696105501930,
                            206251387482263187,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5581150188929121915,
                            15613041204130469312,
                            12593521971388510790,
                            1486725595670459814,
                            8838712029435920939,
                            1750787135667016644,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            173775623303356468,
                            13685044199321349735,
                            2420029312353719584,
                            2548511872438503932,
                            12485350283164324504,
                            1412544822587096416,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14646315801613413011,
                            17771118696092637093,
                            5317808000225717795,
                            14145343188662143233,
                            7131334939058092051,
                            352453884047204994,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10868433151101887672,
                            17156083443970819614,
                            7937247232170699227,
                            15521482206834836952,
                            3773424494027551205,
                            1552632014402865638,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5524756812962325798,
                            759361929155796108,
                            13895715420426182238,
                            7628273294216716924,
                            16509457081153723877,
                            718879206939213122,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13898465135542066713,
                            3617188092800302218,
                            11603146641614077174,
                            5093621330290258596,
                            15759157324378892775,
                            869569389104932276,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8758567428210609917,
                            16885678526976584956,
                            16081377099376555192,
                            14278380459425722918,
                            18159814850406977144,
                            137695384832388439,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            268086193690876728,
                            16552091092254286031,
                            9208806890186085224,
                            6219769294040895728,
                            12570012658957617840,
                            1057942020078076828,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6074194495596775667,
                            15155830299096206662,
                            16101259119622917374,
                            18373574692368155685,
                            12168111145619928437,
                            286167544723456250,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7271620752751665209,
                            11022802743216265057,
                            15447367355557620334,
                            11732155241026167283,
                            12138665303930527930,
                            475638527238766104,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1003180387311734873,
                            17510417030598624172,
                            18027063950908803352,
                            3871593532404139144,
                            16022144506179509258,
                            1749815762781805006,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12673066229103688399,
                            14497929189548215064,
                            11614549541672142829,
                            4595414922277152893,
                            12745465902483169405,
                            1320939607222272261,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8511524649749406013,
                            9794583042370336923,
                            379910379981699181,
                            10633307686876529196,
                            14852336526997721644,
                            1410777649445143467,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6725872407221267215,
                            10040174663012217669,
                            2291717355382406406,
                            13227423857207170245,
                            4822915509310985502,
                            1744811744579650373,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15056804215128145659,
                            8424517300947335052,
                            11496692345500075017,
                            4867542291787385328,
                            9635665559752848600,
                            1551505636687840396,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6846101186532675758,
                            13832619053719231907,
                            4162300891528642969,
                            16724383432324066393,
                            2319342748651941558,
                            1147667831838058659,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8292563806782349267,
                            8301790012444776127,
                            15602178319920051345,
                            8040500567194275895,
                            2678789450244414597,
                            137883935525887921,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7930396497054068950,
                            44428382134414444,
                            5068320004155425808,
                            9709961646554583265,
                            2961328667386644552,
                            216069978353891823,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2946753149072042569,
                            8668781307647786828,
                            6708734244997717601,
                            11020126333496898265,
                            16541571499663390781,
                            1474335972888519219,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6324810725764810415,
                            11944347055139451862,
                            14884795786068263010,
                            13180644548486102199,
                            12871920207488664628,
                            1085681213614294251,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4901222948037995836,
                            9253350513095170147,
                            13561492450749554099,
                            16103412106858822959,
                            9370146867679952548,
                            1544553875030823323,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4898660779608470511,
                            8493417877133092987,
                            13702835158872647855,
                            5711629723844529878,
                            3680958497687599456,
                            1060192575184999237,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            646236390499477086,
                            4073964563500468083,
                            4428385534894683636,
                            6094567376899658284,
                            16160071652588484201,
                            1436119850394769309,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18372827396709910438,
                            14750869468241062585,
                            16916885318022012126,
                            11533924945759516796,
                            1810552611583028232,
                            688306370613501617,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18020249584119733868,
                            15195251662859124459,
                            12506098919330613494,
                            8951358825679545390,
                            7250500665836601182,
                            1056937672772883723,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17862094789221289535,
                            8157722400120698102,
                            5204392810542660969,
                            12407146087858028076,
                            17448914825075639653,
                            820009240697553156,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            878181651966555461,
                            2827309392871424721,
                            15155758762489813114,
                            3337655804304442666,
                            10699116750239460147,
                            541390124425843743,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9994874443423247195,
                            2844636514415479812,
                            6025651419422408155,
                            10485290748101659614,
                            9467968968044140391,
                            539250950017591498,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1249765444576997530,
                            10017444713599648257,
                            4751095969806092812,
                            5079089904441801672,
                            13818193275887051162,
                            1654378821906229147,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14032188997897238904,
                            2554773594843096931,
                            6507939392117790644,
                            5792980338141397298,
                            12976465480269139471,
                            1407532836390758545,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4308177722724292171,
                            17651314459951928139,
                            12188081093447004051,
                            10351271592578650234,
                            16950281205776374843,
                            1459617705166509563,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5297158950579240218,
                            16813055052464236056,
                            10830064416831329620,
                            13722196157006652360,
                            9246319595300847952,
                            953540093009264749,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2807918904136244330,
                            78885605839843410,
                            13332627947027212140,
                            5159415259991596345,
                            15262160841680523805,
                            1240591024803281091,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6675243272447077693,
                            18006057728550986540,
                            5699254238647723119,
                            5729672011656974906,
                            3228053882267489535,
                            1649062278536166704,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5129909721888071781,
                            9092752477352870802,
                            15985054705200528966,
                            5288304962789684010,
                            9811390915461325610,
                            624991152895810204,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            764061627737063435,
                            11510516090839090048,
                            9601667557087190466,
                            11573693605534131062,
                            102067636357319131,
                            137503257236096698,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6644837180613539853,
                            12947941454170514202,
                            2861669105865672448,
                            10489708839068375353,
                            14269234475900858001,
                            519237404386431967,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9224013866331906676,
                            2892554330601488786,
                            11440013869440333037,
                            17168828313346942552,
                            3616023035581198445,
                            1130003263090468522,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            666687315774573797,
                            13028193826424652624,
                            2386937359980419368,
                            12943231552974798183,
                            10522500459346234357,
                            1553146866505358801,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1641894446703109739,
                            9286896584746457935,
                            4384201782494475012,
                            8896773716103273971,
                            9382694630468641237,
                            566984219624798391,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16624347579174681068,
                            562048381020155130,
                            11083198172646851512,
                            6885565830775795432,
                            12579149521723374926,
                            656140515921878116,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16914742315272516182,
                            5531747554354068516,
                            16229508061238155172,
                            9048050390244196234,
                            14991672568044021684,
                            1290259648009982280,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5226989928743103098,
                            16131830629938135745,
                            4039110593285500800,
                            8673398258418086697,
                            3100510133008675430,
                            405848920466426091,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10181418879944970885,
                            6684820937880736983,
                            7313921032468244000,
                            6265814168958053574,
                            5710751572578221014,
                            1635068360013653978,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13420978024692612304,
                            7815419477779242260,
                            11041162735575108298,
                            9543151368073226569,
                            12234352784053392262,
                            97895999631442828,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11751607905087320461,
                            15075470721276793527,
                            5327318946394480782,
                            7983921436713469322,
                            4749890618825080382,
                            362663937458763628,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8319987923319934803,
                            16962096837626352269,
                            6809866037678754154,
                            2251085435364045154,
                            5351422558429667625,
                            40501398144649535,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17767983710528978174,
                            12011908915214498520,
                            6509087700473837418,
                            11246970329614441176,
                            13788975799652838531,
                            142433475180665600,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15575260662712619165,
                            12726649802934624943,
                            15371645751813557811,
                            4207986984477332247,
                            4402016633270072750,
                            1392418701262055534,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7681016974762568276,
                            15617853746082924026,
                            12884096685924261297,
                            260028401916560442,
                            16113539469786799362,
                            1267388871127035424,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15858662236423574194,
                            15168847970851621348,
                            13499610817214345078,
                            17037511167856034412,
                            7765198300560903264,
                            762290233927243870,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            398162318930918265,
                            10251628751891029819,
                            365985343748180913,
                            7598362485699337099,
                            192008833863958468,
                            1032900520324347542,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3140671486602582309,
                            6234278285144488560,
                            8765380575469104728,
                            10546450695513673585,
                            10329441293696541164,
                            1815620277527962398,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            358270601505035525,
                            3547525556145233527,
                            16825145467500526695,
                            5058817127919811310,
                            1272003960759669079,
                            1692101945509907750,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9404700040889086876,
                            8141526065324572373,
                            4560916125519073282,
                            11115280773450849081,
                            15443742421520664662,
                            786656674637874126,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12494186608837715722,
                            14196819732517756777,
                            8548709630800169977,
                            4960787536724909242,
                            3476766208061539513,
                            1450346975077714733,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18328845712550239965,
                            15841172226061905630,
                            7102892710140118173,
                            3827594088236310095,
                            1484947220123014600,
                            842799528108619562,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17882837560489413801,
                            2697631512084633209,
                            11410285113819312652,
                            1367337686699996954,
                            2331690271697427860,
                            1633454247177195300,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6383211374867810791,
                            9310199691179705028,
                            6803773844064978849,
                            16436034629295528810,
                            5602708608480051164,
                            851894785183603651,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6337636100558526719,
                            6270024689566462190,
                            12744202703652335020,
                            9828765686012287258,
                            11042857824159970736,
                            1641697590735289948,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12755059922844064732,
                            3842933527281312964,
                            6840175547142643679,
                            16418054444668630891,
                            3642026116628954712,
                            1075010357252250328,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14903373931043929072,
                            10771971962638112797,
                            9928694236991379542,
                            2546665444140090579,
                            13168896722182768657,
                            1524810020156042653,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10690506755237789145,
                            10351863493084944117,
                            11442631661278390472,
                            9717193028182849726,
                            13649538716391678258,
                            607830192622370534,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9487080889684668395,
                            3464994054160352940,
                            10305915541408335756,
                            4938380227639577395,
                            6318468910487409971,
                            1220246988737233180,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9211704606407539782,
                            18411558539468557717,
                            14091712120718188147,
                            1234074669490960733,
                            3916394679396752987,
                            1608104988263108785,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13611229370564482638,
                            10356037101482062985,
                            18432309519524730782,
                            10072615660586954190,
                            14984675040598092684,
                            1198111650269129082,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16862835211525477682,
                            5614423967069705898,
                            17994304290237003296,
                            10185206470027975238,
                            11010200881522993337,
                            1627366138296930208,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6904716980533169936,
                            5522083241173840405,
                            9702206955325055086,
                            15386622343724327424,
                            15377265206550190832,
                            1291288146596392315,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1010581764698583918,
                            13477453595753129860,
                            13517786003285137175,
                            2698185252897494972,
                            212568387045989582,
                            1191910288598618897,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3848498151561825752,
                            7822503973597565030,
                            6076254401756865354,
                            10847735078876141017,
                            9883377075078835483,
                            531339841517689659,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15417001634945680516,
                            11127687097390465066,
                            5123782143826954958,
                            14518732392918561288,
                            1517478475323129111,
                            1038247759636850042,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            822254545392158291,
                            12578612739097400998,
                            6411611333701254368,
                            13930915737130650829,
                            3407680845962003572,
                            525476616784260925,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16513733824565017109,
                            15951826826584136922,
                            17431517357618551334,
                            6638611513889522261,
                            11090893733323217409,
                            1143245975447070658,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9509197883597843726,
                            10264415293609138910,
                            11169371642408428745,
                            8205885982503875593,
                            13620635475178221238,
                            463585297552670128,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            52414919677958177,
                            16571108006939050049,
                            12959120386470321129,
                            582818224032364702,
                            16489694845019446899,
                            1758328501218486591,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15348154220439911124,
                            9494351055001936847,
                            11413428706166584797,
                            13705531394393840798,
                            13967747373993694084,
                            881170885172159416,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18082041377437735119,
                            13945802348275421927,
                            17155560746771218274,
                            17355907709005851303,
                            386671177415760043,
                            1101405870256554084,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13325333665028862633,
                            6463226842578550044,
                            10945385118157632114,
                            16679699629039922103,
                            3065001301813491735,
                            1655531964513841798,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6620042783646048429,
                            16807165034734569982,
                            17160000155737123332,
                            10073418752785981837,
                            11601104169383958004,
                            997467076302752425,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14618460784391714582,
                            8072192410838700329,
                            4462103291702715031,
                            6451929927910728998,
                            11771107563387159558,
                            152431503207452814,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13467343394026015535,
                            14519960211291222478,
                            6116364849734714382,
                            13688398542554134706,
                            9053392718483088865,
                            1167462993733720311,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8448581603188355159,
                            13266984937238089206,
                            9657442967513657034,
                            12121249192315444912,
                            262245123761882561,
                            577374942525312668,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2207214411818758996,
                            12654130473268507617,
                            3977104691942184916,
                            9024838205932948774,
                            5451590555608660786,
                            1863687486026288386,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7107873856129714547,
                            17198252256560057250,
                            15284848822223980465,
                            15804928380113376675,
                            8366936326304800324,
                            1373103283356059440,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18110536828345177458,
                            13624472210239331647,
                            6689361682488940956,
                            18257864249249281757,
                            15534349310532231078,
                            1422021768962839412,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10767787872910370574,
                            16890762820329617951,
                            2180023298593083562,
                            14070312842136630826,
                            7419627981135724782,
                            394934622900887362,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18417889437257235454,
                            11565377227087440178,
                            16260541837755609358,
                            4485061219711800013,
                            12436044473951107193,
                            640976644042518311,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1187857770421444303,
                            4395428741753565192,
                            3479508211556087807,
                            4905145659377949630,
                            6583122214595540679,
                            67742927980383264,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2871403828040574063,
                            18259484367684264968,
                            3444570270062548031,
                            11802114171821572119,
                            17600153571368458436,
                            516637575294275423,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12471098795256608845,
                            7129995268381337505,
                            8447252122988299941,
                            12342679509488972842,
                            1227451083397175896,
                            1794342206557930553,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15916857954501449044,
                            10803379849704067859,
                            3694701591275356307,
                            16969385224350841383,
                            11373233374542957317,
                            1352367240753911528,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4166244555391574630,
                            2453653214205019198,
                            3379008268327460607,
                            10929836250155821488,
                            14388768655822677364,
                            806153207456434096,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4315866960841088046,
                            5560969776414603372,
                            11045266946809663535,
                            10335775319471925580,
                            12690746112265182045,
                            1305145106642252790,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10582461693322713833,
                            1421651495134135589,
                            5673218480937693329,
                            10867079431292804068,
                            3642304739188457523,
                            1728810185918808504,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9848376785873128031,
                            11177701484982686262,
                            17766589492701921194,
                            13067857598859585315,
                            5985778829411617190,
                            1260383003902580233,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10024946729293106072,
                            16738883806082697691,
                            10010930609323866105,
                            14108970806662829783,
                            11343723084797541490,
                            534515111028459029,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11523738319262622957,
                            7740257256863538759,
                            6339744049776731506,
                            14840741166519676184,
                            706542259598889080,
                            991701450459139222,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2314115340595819600,
                            13898325771599312239,
                            8952213961262281210,
                            2319843824649743478,
                            10119258365144196202,
                            1496606691039906840,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4569053027382756022,
                            16808542297769161495,
                            11644180451975136520,
                            1544029036923589818,
                            9076906934895066721,
                            309588988614570138,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7578356054780461426,
                            13617367227855072229,
                            11194453660702040383,
                            12590510554864058734,
                            3029438799499073408,
                            1426436228144070707,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17283957384820237353,
                            4494835591303435971,
                            15539940631737173776,
                            15168686384549870790,
                            16475285860385070976,
                            1006161318151870235,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1478816007717340746,
                            14014538796467402905,
                            12336091508940659958,
                            7031277857736566963,
                            2854421217255070344,
                            362229673411298044,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1300041852819733258,
                            14621108287456610273,
                            2728568006292171703,
                            13300257463201020955,
                            4719439211793416304,
                            1801073788786922345,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14756481427183722613,
                            2236459028675075376,
                            800934356683803777,
                            17166474772945626109,
                            7392735296653440149,
                            292116860272170572,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3529181534332451079,
                            14785028590225652542,
                            14029703585631357293,
                            6515223415824637933,
                            368400046391788290,
                            1490832248585583538,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5824833846548848848,
                            11395942507388126087,
                            16504216008016843127,
                            3107068389176603865,
                            3984160858027330871,
                            1547630685041163480,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2550897000308849200,
                            4425918360010079498,
                            9413492900770120144,
                            10521469855269549420,
                            8789118225241741635,
                            233087270016588230,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11654130807560830053,
                            12365293757958425726,
                            9113786441048714063,
                            12513266310954477565,
                            11238810873360029693,
                            268563522197316187,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5761559293749561305,
                            17429803534604531027,
                            2171425178302867056,
                            13297820184467406768,
                            15287872665619137041,
                            83232806340482704,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13308270199210710103,
                            8756922041902009430,
                            7217587539036245026,
                            9757693488256261677,
                            640609962995055469,
                            1700750094725999395,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8789782568823966391,
                            16554542327134432683,
                            9942611966892757186,
                            18317797438343030860,
                            5426573831258621048,
                            1774077237922160223,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1971903592417757003,
                            9051380446086213612,
                            4879225538680534183,
                            8000843671497682360,
                            9238958824761543848,
                            1022887598423128364,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16950704595241643804,
                            8454367624401995273,
                            9641663680129882690,
                            17482647574971164306,
                            9457701009293008259,
                            1377874901933196392,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2628553885832069024,
                            9323050185443751273,
                            7797393169380040750,
                            12259715252507814163,
                            12243141725798862591,
                            1237259886829163513,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2019793253152780762,
                            6208584158529324747,
                            4156915506134058339,
                            1051781123635220400,
                            1675239580451457498,
                            373864107849704445,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1135526659102842556,
                            13947618642946041407,
                            17143955487804976059,
                            6599715072713151566,
                            2831343747770372790,
                            1360583162082452246,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2420739821469435949,
                            9572031620790523894,
                            12705897566129679198,
                            6263459072032811217,
                            12398224324953892060,
                            275953161232432981,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1807449594497835798,
                            12072039966873120184,
                            16027980004702278365,
                            5647517832333076176,
                            5224296596512541178,
                            518623419643079104,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14450819836747970954,
                            776908227960242261,
                            5036100075309736420,
                            1748971143197513910,
                            11403707966513290413,
                            1716098299983364952,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16470746187821406250,
                            12477055931596929283,
                            1439195669684951613,
                            11835205415092796362,
                            3769621740155449050,
                            1174058899073253899,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10361001716200388234,
                            15913774155802301939,
                            3832179598560421642,
                            6631658669006131562,
                            18231162779254170103,
                            153978153231651036,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2532816590532388285,
                            18418007074623057836,
                            14876379914049171252,
                            10949798261286706323,
                            6450199992500703418,
                            1380866928858766288,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3505409636955575335,
                            11659803657327424287,
                            7928183387101122045,
                            14432429507089561837,
                            6747119498675511352,
                            1512910714180303210,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16072105086373277838,
                            11264716059029615762,
                            4863790618384846308,
                            295556596306197760,
                            3858927471229060521,
                            1857554617651525518,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9709323546952538708,
                            6128731605552925548,
                            5193807813976663028,
                            6440271142981778710,
                            9398016188601800179,
                            72280616411116262,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7308037641079580339,
                            15989426284036336247,
                            2283786473773907257,
                            17880162482149852100,
                            13728632358043475279,
                            33211343759004118,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17709760115528776667,
                            13458947110973479089,
                            167375196546327497,
                            18337240012879334191,
                            1617865131254012135,
                            1110587551801850192,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6894739809028020557,
                            9954581358854027137,
                            4505957568602194850,
                            2372360724766210391,
                            7583674635909314149,
                            521858049319865698,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11751633987849725502,
                            3824150350611343807,
                            15763953394925035681,
                            3967431297660772651,
                            11554991520440437079,
                            342747916109663704,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16269453512264180167,
                            8284925182658806340,
                            7105068912916409864,
                            6846470837196586123,
                            2798785993790409964,
                            1535776915658459113,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1347405218661338308,
                            3616705963637136268,
                            11808621222802927910,
                            1977790853925079136,
                            12474755774999526375,
                            1682566796545772916,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10970491095365911776,
                            491180423110296350,
                            9145625143541587813,
                            7780656760370166253,
                            8609508757375954535,
                            1074763081940305279,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            273302092082609265,
                            11288320152427334948,
                            10196786423941273766,
                            10398531016481890397,
                            13885486983667773314,
                            1406536252404748128,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18163459427631113928,
                            17620650646384238368,
                            15242494606726597733,
                            5124676651866623044,
                            2477312505047751882,
                            1478393518682627551,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2635816435284932509,
                            11193841437421194152,
                            17655621614795434407,
                            905881093666135867,
                            14732045110343100699,
                            1260179273723144543,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15531923520776689042,
                            10939417411215210875,
                            11886581412960953088,
                            393351792223882709,
                            16958872878598443557,
                            1840689855787161061,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15411677510323232837,
                            8011416456004692033,
                            6451056347125552952,
                            17826317509324730949,
                            12979653557694797228,
                            395891008762203795,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            902243416528007788,
                            12307003495260170888,
                            16941368428221203558,
                            17093763012717962522,
                            8861107750417129037,
                            1845652010162269993,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16702302197264388687,
                            13329784506862280391,
                            651190366820980542,
                            16954646127597446905,
                            16372374756950421112,
                            790528752435626277,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9832582955612742606,
                            16713664150433920230,
                            11630601031307185658,
                            8225624013403821048,
                            5533108273241750219,
                            488870696058317489,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7093757205524807374,
                            16115333825248257926,
                            5432286634366508181,
                            4746549143406321796,
                            10198645512208316053,
                            1463351733819343525,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12562653642027112829,
                            6153133235628158883,
                            8503270406043932801,
                            12133093469160874603,
                            17490921749119206133,
                            1694383758542141690,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8328307160671734367,
                            17025186070726567227,
                            161391150224795321,
                            6964690415163198216,
                            18179471273604771651,
                            742999165608141064,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7138840994162046524,
                            18092231953034477841,
                            15371600124197746589,
                            8036440365446923086,
                            11822833741850143712,
                            427939719573436143,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9866847913847843239,
                            2874011828241588089,
                            5411182748451114530,
                            4056300487272846036,
                            4253559059574052483,
                            1310715453106327508,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            921376483356364081,
                            8904855143264868467,
                            5131353141241161696,
                            9338863313790827235,
                            2448239921263245284,
                            218736114241983843,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7877317291112105085,
                            6383554603033964313,
                            9314087459312335304,
                            3827966683338148268,
                            16095519809653084297,
                            786683255175591043,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16717889742354187264,
                            11621793570749585829,
                            2979039280760479685,
                            7558335187962575611,
                            14665134273371575317,
                            1651842112567288734,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10230581729368344345,
                            15017446822880318685,
                            1352520305374122990,
                            12484100770319536173,
                            3157752342743161442,
                            1543267160227254990,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14066274715231130710,
                            15471623429652580653,
                            15910143342803994456,
                            7675753009521167797,
                            10801262376768541950,
                            215532025390617006,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7513852181006365661,
                            4347005562308066540,
                            2330123869934145226,
                            4148169022910607011,
                            11803172665265736828,
                            1681999729986028127,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1686253487944201345,
                            14459263991888274587,
                            2837209461939537978,
                            9562714092221938966,
                            11810942071253350769,
                            509386285145095090,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16896120013345922063,
                            2015412759505212505,
                            3775655609227193108,
                            4306189755620296098,
                            8810287271482684867,
                            661570206385308111,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4943868888310011873,
                            640553403676780318,
                            2272312131426990210,
                            6674336632444905071,
                            207193693057578159,
                            574494455884395580,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13444301857312211944,
                            13307653527912892756,
                            8259656047107840248,
                            12241408775735668264,
                            8646441624791105022,
                            164651162342091548,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2914147990781397770,
                            13464155694463248656,
                            7893943030204674795,
                            13878599743243780837,
                            13984194953390596069,
                            457715547300131728,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15968752406305239378,
                            13196173160477609205,
                            6829167694882396160,
                            6399562110932456156,
                            7112087057133942162,
                            85359274720893104,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15821679821104304720,
                            13661630146772471319,
                            12540702131337096500,
                            3187807835295994566,
                            17995618927156352188,
                            156140200610092966,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8880351356837985561,
                            6077554441802302267,
                            12567493849486885145,
                            5688947788839532304,
                            8324143845537458856,
                            833820836060649643,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4725217272230715017,
                            11166057522677408116,
                            15346334818174374510,
                            14559767943363177738,
                            14526328453107513203,
                            1702697747204937618,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            999808702288960074,
                            11653537408947295076,
                            4441122851485248500,
                            6669856488833190039,
                            15056649560122044917,
                            600094219550012930,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11016139238069432380,
                            12886053514613261224,
                            13996042521099085741,
                            6684507750598416381,
                            211236594472132863,
                            111324687765614382,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7827867860968150508,
                            13709715460588626967,
                            773127136413142573,
                            9866229592328774359,
                            1995958895942128737,
                            1577004374164239863,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1487089895634605300,
                            12329824240642153694,
                            15717799266468408322,
                            12268452327606578417,
                            16957960556059609112,
                            1840205402176869410,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            273940847073639940,
                            9317711995771492933,
                            11939310581435867123,
                            13384257407375693468,
                            7379720874751054366,
                            586920359916203135,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7733087565519903034,
                            2714702537414862647,
                            9001781259792699600,
                            8670452907289762856,
                            2674476231026785805,
                            1510535181681380993,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13203308686520666544,
                            3697976354806753395,
                            6819886112471389274,
                            6776341440202676184,
                            18189686687518441333,
                            851284887294549976,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12747898024142236476,
                            6951599814704662131,
                            11169010306090335279,
                            6879691424378406656,
                            7652253764784521238,
                            253334100448444524,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16446676748251266104,
                            10082276747657428134,
                            4353878644809993783,
                            7516958613202366348,
                            17042291501041099643,
                            735942598773117833,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18131859437184299635,
                            4707500528900736502,
                            9173972144964420486,
                            2217797159770472244,
                            13505583623193286023,
                            1278487444518123476,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7235387926634379098,
                            7506553406449065154,
                            842733387743077064,
                            5604021012948211229,
                            13627376142779118090,
                            1364465935712553921,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1239791519728426771,
                            13626757135229156293,
                            10124840788609655278,
                            1735796870948222103,
                            2551874351407039554,
                            686613751297443281,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3143680729994791809,
                            1981199770166695218,
                            4742256039787364846,
                            16378087098423789095,
                            10670182922377613692,
                            1664884387466088584,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4868629058947666502,
                            16403082561971213430,
                            17110385730293127081,
                            13990649526585914687,
                            11068517966108566119,
                            482471922991500900,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1640386564514583591,
                            17706262096160842184,
                            11423531618934968565,
                            7469177445646766004,
                            12190046234484419867,
                            523257893686507424,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3314362024773756315,
                            3157458906761065275,
                            9949531380843452482,
                            4420517121638693627,
                            13469709788440962749,
                            1582551223257480893,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            850477398521361634,
                            1226024139617458148,
                            14406695314341069160,
                            9690753824858676077,
                            8844003618902991368,
                            257209171933590070,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17599997595449443990,
                            13423466953922788131,
                            8464899228622968383,
                            7886824424197144308,
                            5884380370285466669,
                            1633756787518280552,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12488511013179109847,
                            16182849604887475606,
                            10869569917260546107,
                            3883906897773232022,
                            3786773448390666059,
                            1715816396981428705,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9156869255291287657,
                            10758605500044185608,
                            422553296392574026,
                            3150952449875928287,
                            6253737991241622494,
                            177410464059451242,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9203355707417622557,
                            17673548030136837312,
                            14829851992080343769,
                            1489448167137871810,
                            8156160076446036564,
                            664057040378237426,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12238853554802259048,
                            5401570296405622550,
                            12775672749111861713,
                            6451600090633202818,
                            1299844572645007964,
                            471782713435473731,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7116673912154464983,
                            6890666270866097154,
                            11523525191668709372,
                            7858688801986355354,
                            6691342903745152029,
                            1272800137069087603,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8147986179841197490,
                            7037834274532145888,
                            16513667397917687242,
                            3684753190796303960,
                            4319291724066268920,
                            570638225356986820,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10472758233008575058,
                            8203407871264495081,
                            862273195455646918,
                            12296962400093884365,
                            2433941693624358179,
                            416427949986411059,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6346768162266040504,
                            12908051617675404232,
                            12015246469850114469,
                            10360310874699368184,
                            16277727464737162303,
                            776277140478594879,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17132544051798434927,
                            1502480067604325507,
                            5675558064925721625,
                            6277661675936686087,
                            5448140404250629557,
                            1677148357499647005,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11472140242462146808,
                            3028247269348704778,
                            912100754944147963,
                            10520024616482710456,
                            11554649586339460021,
                            729394326686031929,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7849952756476637454,
                            14836128991949242091,
                            3164799842095240837,
                            8539118838606103990,
                            15417600034333421456,
                            444714657939925424,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11897685078271997898,
                            3979793691320478104,
                            10635814669099757003,
                            13508541148396749490,
                            23456926366438676,
                            1508789501131098114,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            725537276168582567,
                            13279020612273236920,
                            8600503519229662845,
                            2177687990414282083,
                            14481720689263504093,
                            672717684785105856,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11877488030569984235,
                            4606734668180044618,
                            10200219210635676961,
                            8754906850011424633,
                            12241457073894028662,
                            1525390095616836834,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7601639232399030014,
                            6753710675237660646,
                            9794594093496268882,
                            8211575312494972654,
                            12236998915761720516,
                            429376505455146738,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13455466768492496324,
                            11933138028659254740,
                            5742193169261664739,
                            1563937210027874184,
                            12627565421707921683,
                            7607375195881987,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4472628154608418474,
                            12450401936546064011,
                            8906676456411017829,
                            9054091377596680601,
                            17715119733273911311,
                            666426954173471519,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8668280722600209713,
                            17005470759847722974,
                            314289608437059049,
                            10120739948457627043,
                            9469830028873656960,
                            1435056595383618816,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14307937008088431853,
                            13928445934998666935,
                            10639493752946613990,
                            2228189168206579526,
                            11194494826006038460,
                            1697434480657476869,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9156021816844158820,
                            12952379316085467898,
                            8052544463366476366,
                            2423485466288300663,
                            17992569736400566586,
                            1344230883382702091,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            463349153552070258,
                            13180214413375236763,
                            1811105006019531030,
                            2086652870695795810,
                            2804160736430674653,
                            271255611018440993,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8074036664553918131,
                            14504155619738471703,
                            15955230564717103906,
                            9926205275366045464,
                            15300967417538591191,
                            826894316073335139,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17712605827481588017,
                            9409548113693686589,
                            15893566436708991972,
                            15464681893979410272,
                            4097430218971375579,
                            1078521177403778777,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10667412366341817961,
                            8800423225125185141,
                            15313799842929688,
                            4347341023889040717,
                            10925522359050011639,
                            1085379683691922914,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            18139988860053041321,
                            6924956073732397977,
                            3332474035774308525,
                            7525948743673931197,
                            6530409666150962961,
                            648583640087129521,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16708692049433933452,
                            3704362550268671315,
                            15397897714466331873,
                            8796601966776907507,
                            681184695804999468,
                            103773565434204354,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11707492670116299961,
                            16235969377071942358,
                            10604511742965759751,
                            16772824760598781859,
                            13533709776709192242,
                            1255064333890870785,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14002404230897725169,
                            2048840499265678734,
                            15819518809772376274,
                            10789708341389556526,
                            1277665599617264290,
                            1103357742108997903,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10517125507059116897,
                            16806547559419822641,
                            10877113035762219109,
                            924542467244004018,
                            3066828346619845599,
                            138250838830239327,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4606893069377441602,
                            14595932593423141097,
                            3162678333226302647,
                            6614283208277980524,
                            4505995971867519283,
                            1560847878469161637,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2989462488832847337,
                            6014060526861555223,
                            6034898806531525139,
                            17524522156494237629,
                            10475882534618484881,
                            1815701618185474125,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7034483930114708060,
                            10673098338846687236,
                            1976827414659344154,
                            4350952234990922470,
                            16101002989440050674,
                            615607586626774522,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5730332253645086288,
                            7258740662575138060,
                            5697092052406480296,
                            9632902895066875414,
                            15535204966655247582,
                            1571136775547144247,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12899067055708288314,
                            4443256329076763665,
                            12642170914166340899,
                            6901977068799990571,
                            15620689885070924537,
                            1130553614992758459,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10857485131992628584,
                            17503751600886613249,
                            10303782554794108357,
                            8407645970255525190,
                            11536477980761550447,
                            20096210566812068,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8456955960483856687,
                            504119307649462889,
                            6738680323214540601,
                            1060325156758868224,
                            7224629145319592707,
                            1498193062684977843,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16345753462906147838,
                            7498383959421992279,
                            17029831935643470086,
                            13708612483506376477,
                            12450288622624091964,
                            1790194938355402421,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1083158896825713243,
                            6021946106756788125,
                            4469228809221934372,
                            5863031858435639779,
                            12478422863957141145,
                            1404103260604143351,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16109728002770477265,
                            16150876314766411972,
                            9025248167612410203,
                            1702552369082553207,
                            2456830397440131140,
                            147883535557300333,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15741594309865198427,
                            12844824008626352908,
                            3396890556579778846,
                            7870983673109773650,
                            13267348792570502130,
                            1480636970106159031,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9146931771288393191,
                            4917759796730437231,
                            13774538674547011708,
                            2412369411114492794,
                            7532143508750199827,
                            1715653920503912371,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16733551056434501112,
                            7758181290713139029,
                            17274682555689820423,
                            8063340014223902367,
                            10374722249424609317,
                            1532636191410158852,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7469983737984262576,
                            13444162346299449071,
                            5640093396606158052,
                            4930134741729356561,
                            14351178775664990289,
                            903744531402739160,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10921820192105236531,
                            5710831249936970604,
                            11883553636382379495,
                            5329528963048660501,
                            6389474642049074655,
                            1227183968935239089,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8268458218560045735,
                            1247747948912575974,
                            11309995967745110659,
                            221412634410368068,
                            10511326602031662205,
                            1356672976286742493,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8851811869251792960,
                            16852567963343802301,
                            8477822487198921232,
                            8657015855325168748,
                            8356960315330523120,
                            802256233794573856,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6838860473957403922,
                            12313877437669464187,
                            11153687972978960664,
                            17520606900287398684,
                            10955047001174645291,
                            1277185041053943198,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17974217981234414389,
                            15760668031679006843,
                            4473082252882836184,
                            2472343102452889228,
                            4217629566377302317,
                            673939370591314449,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9547197942153139461,
                            14599218097482818187,
                            2843424473587308532,
                            4226145058210495005,
                            10528402275553705847,
                            66122988350080075,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9174728851225898486,
                            13098075324649165694,
                            10439813280218057912,
                            4149789530302721259,
                            12467003151662508860,
                            559280030928336852,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11925776125368288844,
                            4594781337306308900,
                            10343994527611316053,
                            5321523789209385989,
                            1468639944565805316,
                            1716945381951747461,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9309706874336969640,
                            18249156080332192890,
                            7400324714245133725,
                            4600715183174911203,
                            4434458543846820965,
                            705013663117451900,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8161792535592809111,
                            17180321596014781518,
                            11110225832036956334,
                            12794711412350872121,
                            3998086829560658135,
                            410530957324496823,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11158746874715176214,
                            9208255443169355771,
                            3525806261835140759,
                            9263604495479796005,
                            9267449553772993635,
                            1663084033262685090,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17230894163406706274,
                            3344852640084939849,
                            9408400016641730870,
                            9520580550898937799,
                            11225681528642350956,
                            1454837095430890026,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6465044866426696949,
                            14736799700080398479,
                            7329547077411965229,
                            13898441080210442724,
                            8586763158249649979,
                            1272685349044293355,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6379549295073248583,
                            16758432761952000119,
                            4938155452083238053,
                            15340181483282266020,
                            3150190246406712413,
                            1521258000049588256,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5572866548566351342,
                            17927456433915292456,
                            7591578908053469326,
                            14441693502703011309,
                            5804666988297494132,
                            807982453740109106,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7571107375264872895,
                            12730319594500725330,
                            14640450400108196727,
                            16281298608282915713,
                            15003677693930998301,
                            1447523451147133719,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9308475065587204712,
                            2769831683736697891,
                            3081672277450570895,
                            12710300441878610747,
                            8736197740052817053,
                            1082428885685039865,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4053285363018068426,
                            5295934290818197343,
                            2359295125788737404,
                            11506207648153771931,
                            9453312492117080046,
                            1522713550491505863,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6530972073717980429,
                            12743165011542503462,
                            3789082390943235041,
                            3957079655578167314,
                            3633564338851865523,
                            1058318646105736954,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11427697266105918501,
                            12031999123617252622,
                            15152335280862710845,
                            3577049759309012956,
                            17469102907553288838,
                            173547675806560419,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            754085458948058147,
                            18042240791334696580,
                            7121536542901390847,
                            13809093444230709261,
                            5463424614508194633,
                            1629764644556285880,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17994310442554117752,
                            10107427397169532768,
                            4025118507520359153,
                            10835594492169326674,
                            1448813335077624154,
                            333241649140391369,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16610706687903488718,
                            1707580806403275446,
                            728494585461945183,
                            8148665692689227089,
                            5643358409827969455,
                            543217172092246273,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5776130648454740791,
                            3255482387510895526,
                            12847710318252084872,
                            12418294285422967743,
                            17545714928967286236,
                            280833451084954420,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6198987211828323893,
                            4888250708161812321,
                            8966130244611407524,
                            4860040864074970623,
                            6730031969028530820,
                            1330513539224022817,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4534817001571825772,
                            12595716462508300135,
                            9245703737032421130,
                            16555921426334908167,
                            9149713329238225679,
                            317279687175405929,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16563925515999609675,
                            10531842265939216597,
                            7924226105056259118,
                            18194235837551438752,
                            11969680315092058080,
                            779855209608535420,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2285333833311126570,
                            18414073996087285656,
                            16510222202050493027,
                            18045308875833994807,
                            7608031966827369797,
                            1446431002718582921,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6253016062559400158,
                            1768983528656753799,
                            1015339732087435126,
                            5228119126441997756,
                            1634425212234871080,
                            920908828571853283,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4500701681989414853,
                            6227194851926023849,
                            17018201798777582441,
                            5766400526536322294,
                            15759904084295137794,
                            1125285327242384625,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17369365987605149548,
                            1434713199030345536,
                            7070005886771227291,
                            7905328907727893782,
                            11803141667835592928,
                            1017833926334691231,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15157767008353352235,
                            988448996769034773,
                            13072216551743013591,
                            393705439870790308,
                            15781442629164416747,
                            142693179101010845,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10663000213834161527,
                            1460073168411595726,
                            5540896151500075742,
                            14684908760323824929,
                            11461785783146903473,
                            1857823597189898311,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7358969635759224566,
                            11841563867005616098,
                            16344779448622422489,
                            10472968046889294191,
                            17546668645014862916,
                            515919024375442511,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16955515099411435689,
                            5067337249753307921,
                            15583505317958350893,
                            9305526825430761149,
                            8100831237909139246,
                            582867427597403813,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3681257299493616733,
                            4826659180959162233,
                            12656298562926018191,
                            16792924134630657021,
                            14651615011977838950,
                            1565335433877586795,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13787786996709334901,
                            10975114728941621457,
                            124584769900005169,
                            844059334191713034,
                            15939624550526366751,
                            1463726640250757497,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4443506881466555460,
                            2956392977786541522,
                            8220689090610529475,
                            17357507452789258642,
                            11868148611142839928,
                            178794679203704454,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14248296018633900296,
                            4126975261765305621,
                            13827746014159220025,
                            15448123387090998618,
                            7542794996450245000,
                            368205065738235697,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15720122355877725430,
                            1582389844810680393,
                            2199527486499290972,
                            3016405396344309126,
                            11180079832262800109,
                            1030886821834642714,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17796355467397790947,
                            12187717252760210269,
                            3447012880927566941,
                            14823908506665314409,
                            14480765944362515404,
                            363107683874242149,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3548281729376862672,
                            11311470420872033740,
                            5302547431284724156,
                            16835127487048093637,
                            17133853903532575805,
                            36712461652184054,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6698401212490142508,
                            15487762742334237664,
                            4984389744721378051,
                            6009671930720933469,
                            17157258401101446749,
                            1724686059218506474,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15844237932345543796,
                            16186298051791839228,
                            16566643060018305018,
                            15276968832228345858,
                            16620567767955680473,
                            650222433851520260,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4785564359001527831,
                            2909823080243461215,
                            2753169488332247032,
                            114888029710444669,
                            2030653512856828996,
                            549214194084709772,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6631350357785754962,
                            16666751545913807032,
                            5773599724759850454,
                            4048931462191968268,
                            9396947479687796223,
                            600370974478135790,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17960576473717921540,
                            14512205177801195977,
                            14282583459249268121,
                            15530496582300288464,
                            9472491823924114365,
                            1050571418627084176,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15211666919795432067,
                            8910028194059406707,
                            7937715742839600418,
                            5620794799648192753,
                            7671348663241508550,
                            1131803267913424091,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13275786633886416347,
                            4711744178323869697,
                            10776722311107389904,
                            1232839793409698212,
                            4321320686480936824,
                            584456672493082073,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3040411644167830961,
                            5860445139672575436,
                            10672887391475300698,
                            9652017140410942632,
                            14894169694358197969,
                            979298791077723251,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4560400612561866517,
                            7843096702520040146,
                            9382052892317184037,
                            4927770526942548355,
                            14211807329986844328,
                            1390365408084295992,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1212515474621441072,
                            17499562995772434632,
                            3281603427005694766,
                            5681209058719508225,
                            11517712904912006270,
                            1765272408863006524,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14317232285583823062,
                            17617236288766523936,
                            2490656453904653705,
                            16399230600342409727,
                            10504291162066533115,
                            261596799080919077,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16735422357982411172,
                            12328870570985204695,
                            5462012185289186183,
                            5063533723672183382,
                            13122159980109442292,
                            599097654935426055,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4636258825827132770,
                            8777126499509071323,
                            11426336185763660458,
                            1323572677627443622,
                            13178701509642075845,
                            1847073006776399214,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3703435513976449794,
                            2427920074584593681,
                            1187001887440090597,
                            17929725113317508096,
                            2553586082896065831,
                            1051895174957439116,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8505375425920060127,
                            10760409971215563764,
                            7396904385399139461,
                            9398730898180591323,
                            67274873258629391,
                            432643262860193069,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8889410584014077973,
                            5057374905924467754,
                            14535209226845822288,
                            11809906469035282107,
                            11835029918342672279,
                            230949379007949368,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14385603525363571027,
                            8263822703552057223,
                            6906780269319869044,
                            12812025926571262286,
                            3329299218355856650,
                            902342746687733742,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6381480965783195886,
                            6801982685934082584,
                            9552723459907582945,
                            7227621655766437663,
                            17811978417029793192,
                            846117289818615264,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            975726921832491674,
                            11233079937918564088,
                            5481001837594413369,
                            946276207425882044,
                            11599909668745395529,
                            1295288560779515403,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            628270652034388064,
                            13071978312974934462,
                            12268988975586922342,
                            1365403283267513221,
                            18403609400736329894,
                            1009240925248173493,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11826331766538704116,
                            11531226093389379047,
                            9111665245447751820,
                            8790339655240654640,
                            1936904705935344705,
                            924136873903360319,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12649374665513263523,
                            703214327952916357,
                            7352584555912960209,
                            4565915610365726442,
                            15154213348689850867,
                            1007834867915836837,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14693490133877598167,
                            8615264843200990998,
                            12556985069971522790,
                            12400284901901091327,
                            16687750692498812734,
                            1838204510003351877,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1826791330213159679,
                            11684022647749726879,
                            3108142437823195200,
                            4934346929776005528,
                            4604210943387210687,
                            1604572807670231444,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1462506450014411263,
                            655758990936382180,
                            6148871309207543431,
                            8714662321385374408,
                            1098599797420132952,
                            1060496077950246492,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            2563436361517803729,
                            2086304706931456414,
                            11715193541827493647,
                            888198487856615222,
                            9207084805341545155,
                            174027670248643091,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9827454925245405832,
                            2973488614315490117,
                            2461724470034388780,
                            6780842959972496884,
                            10622610979084267003,
                            353899653154493117,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12676626184817416026,
                            8292384690968177193,
                            17430451618179891712,
                            408055503507517563,
                            18404272107617027909,
                            1182778590049801251,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3157712026389725358,
                            3544853804642685428,
                            4514459677514929764,
                            17823790537699770739,
                            12164392682423297797,
                            820216143636894779,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            13956408150969688229,
                            9451298002192353074,
                            9379724752019650009,
                            3690734344453798861,
                            9581937276871546016,
                            330273375435908480,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            15938800307609680076,
                            4073580330103918405,
                            4363739710177589984,
                            8789260225996336130,
                            14177230097139219620,
                            1660072885847588159,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11331303573192847712,
                            16203679867609638099,
                            4754011587778013520,
                            12915674019060459994,
                            1934468492811139780,
                            925523587156519663,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            14360131041954642393,
                            5556414396717581453,
                            16197214716364322629,
                            5415017693890800849,
                            16051715855394774104,
                            5926209182196985,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10026595668372541439,
                            4882871068180769652,
                            3389278771160120158,
                            12876578977130711726,
                            8244263136501471427,
                            338836335078967683,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16978885488928895217,
                            16022136308687245155,
                            11833373713005267618,
                            14867193813911897263,
                            9555372035682819694,
                            556931412857622436,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            220660142471319347,
                            10920215827626514177,
                            4241488085351173849,
                            7798905176976138206,
                            8669723379235868712,
                            1462936359634599539,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            6243503383328877837,
                            5727578834340876985,
                            15411217530341751300,
                            2353084929975756036,
                            13251693223881612765,
                            444726686851215067,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1517633259641896216,
                            2398309514230827916,
                            6779465744513889562,
                            2651039224042114000,
                            5382010513811017680,
                            1720010634583036866,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4169553049677853829,
                            5361009914769511499,
                            10119034315274181299,
                            18016282188849693057,
                            6767807590206143173,
                            1022578163456845962,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12057857944781906687,
                            15591744129202116859,
                            2132656656635181080,
                            17009067558206246355,
                            7118144858046480585,
                            792848088863435438,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10303366621885392257,
                            1307897183080767395,
                            5759698657122936012,
                            5642011229054825968,
                            7449027764383047374,
                            838145745014431619,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            104876992545211052,
                            1471350652733741482,
                            14612197389809315695,
                            10191063893926040701,
                            73225968401856968,
                            297425489642142734,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12436425181864991594,
                            1004477940629415688,
                            4815289319108558801,
                            10750531686397049914,
                            10704839728835215677,
                            732237864471822729,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9681097773874003812,
                            13321356925228721436,
                            6572430093742575873,
                            9163058826547466559,
                            15240346725831411024,
                            1703825983307289059,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8280304832243563177,
                            17986463224104421364,
                            2025090693591655146,
                            17033354531853814807,
                            5618237026847530436,
                            747879086889921948,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8642200438376305573,
                            14303195194181054287,
                            15563936043761425331,
                            6917829317234673492,
                            16200076384395373396,
                            901465699924026374,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11362883218285611559,
                            17819939515262828627,
                            13727126749891484254,
                            11310440304246503056,
                            17404812200047194810,
                            761501084855367490,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3018452406225875855,
                            6683238952834015149,
                            16872005001676025513,
                            15769126677443767377,
                            15577069375642124732,
                            355826045107891519,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5946829177584488917,
                            15949957918290496953,
                            15073412346258939909,
                            2775952368836878325,
                            6632837609711069911,
                            601170566626596795,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            11861346849605103917,
                            17809980588897642557,
                            10747901601473276829,
                            3402741114897080533,
                            1762906362552051044,
                            1426712533373456627,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9937604969890750199,
                            4614668782331763646,
                            11349996676328764269,
                            11668104715133397677,
                            16800513839194531786,
                            1258294751596811946,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            5052176037179002848,
                            6980979362516030011,
                            17668920266962814733,
                            10889348687570753558,
                            7874824218173420371,
                            1594647829750025752,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            10590331424924699487,
                            3593841465027830783,
                            6202841319371080366,
                            754912769073075899,
                            4602276352583923740,
                            312561330974708198,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17380423081289664267,
                            4663446029249487065,
                            14093773292687509780,
                            526952317123990329,
                            354477309655992936,
                            1251842376018489055,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            16115314256723127776,
                            17376371271663601819,
                            14764871545058333610,
                            15748381964306313420,
                            2371563845675854682,
                            468236638430983773,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9519258557963545623,
                            10218805620159700249,
                            1734371048176574199,
                            14667465644903844798,
                            8827837969750605053,
                            457476084849609844,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            12180089775124528777,
                            17692306419441560886,
                            15562424393493290067,
                            1112141997765877690,
                            10160371303951815042,
                            1039629367900369726,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            9421164036044390086,
                            279020753926020825,
                            10725812785889227206,
                            905571621272799120,
                            14814457884764197086,
                            1588792423425743363,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1674383127733047014,
                            12633139798948271696,
                            17344989456241582541,
                            2386419018435288287,
                            13625834384214223916,
                            191689017744474030,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            1462838877538247708,
                            8464432251473953558,
                            13827599480777523525,
                            6534154372025212797,
                            768433104806516168,
                            388859280029941669,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7272604296928691358,
                            10077790980595287304,
                            1562683634096076231,
                            3269222634621611106,
                            4238700277701255352,
                            916207750433263517,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
            (
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8386313369719099530,
                            12194282181274218945,
                            18360985931761351216,
                            3540365973775214649,
                            4184998451581233512,
                            518150923124624951,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            17578227858366831484,
                            11456109190348289421,
                            437599011680526949,
                            6168402616437002213,
                            611200559863145297,
                            98610199661395868,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            7945315586259569061,
                            3273063287875655232,
                            7173238583905590061,
                            9610239530230900185,
                            7221327398198084279,
                            899270980372471388,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            8731211054787217311,
                            6384331869277595912,
                            11680039171373002168,
                            16304385507843577442,
                            9069083726310112127,
                            936897104328785261,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
                crate::ark_ff::fields::models::Fp2 {
                    c0: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            3703388115706493751,
                            14895905089655133838,
                            18419205411682486641,
                            11718932807822355454,
                            14866683930838041711,
                            779494058690467402,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                    c1: crate::ark_ff_delegation::Fp(
                        crate::BigInt([
                            4904201642943464699,
                            12387695678074990063,
                            4729241729481440363,
                            16884602615433592440,
                            6411054573889690659,
                            969148383752546584,
                            0,
                            0,
                        ]),
                        core::marker::PhantomData,
                    ),
                },
            ),
        ],
        infinity: false,
    };
